#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	storage::IterableStorageMap,
	traits::{EnsureOrigin, Get, UnixTime},
	weights::Weight,
	Parameter,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use orml_traits::MultiCurrency;
use primitives::{
	arithmetic::fixed_128_mul_signum, AccumulateConfig, Balance, CurrencyId, Leverage, Leverages, LiquidityPoolId,
	TradingPair,
};
use sp_arithmetic::Fixed128;
use sp_runtime::{
	traits::{AtLeast32Bit, Saturating},
	DispatchResult, ModuleId, RuntimeDebug,
};
use sp_std::{cmp::max, prelude::*};
use traits::{
	LiquidityPools, MarginProtocolLiquidityPools, MarginProtocolLiquidityPoolsManager, OnDisableLiquidityPool,
	OnRemoveLiquidityPool,
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SwapRate {
	pub long: Fixed128,
	pub short: Fixed128,
}

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MarginTradingPairOption<Moment> {
	/// Is enabled for trading.
	///
	/// DEFAULT-NOTE: default not enabled.
	pub enabled: bool,

	/// The max spread. The minimum of max spread and pool's spread would be used in trading.
	pub max_spread: Option<Balance>,

	/// Swap rate.
	///
	/// DEFAULT-NOTE: zero rate if not set.
	pub swap_rate: SwapRate,

	/// The accumulate config.
	pub accumulate_config: Option<AccumulateConfig<Moment>>,
}

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MarginPoolOption {
	/// Additional swap rate, to adjust the swap rate in `MarginTradingPairOption`.
	///
	/// DEFAULT-NOTE: no adjustment for this pool.
	pub additional_swap_rate: Fixed128,

	/// Min leveraged amount. A position cannot be opened if it's leveraged amount is below min.
	///
	/// DEFAULT-NOTE: no min requirement for this pool. (`DefaultMinLeveragedAmount` will be used)
	pub min_leveraged_amount: Balance,
}

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MarginPoolTradingPairOption {
	/// Is enabled in pool.
	///
	/// DEFAULT-NOTE: default not enabled.
	pub enabled: bool,

	/// Bid spread.
	///
	/// DEFAULT-NOTE: `None`, pool owner must set spread.
	pub bid_spread: Option<Balance>,

	/// Ask spread
	///
	/// DEFAULT-NOTE: `None`, pool owner must set spread.
	pub ask_spread: Option<Balance>,

	/// Enabled leverages.
	///
	/// DEFAULT-NOTE: No leverage.
	pub enabled_trades: Leverages,
}

pub const MODULE_ID: ModuleId = ModuleId(*b"lami/mlp");
pub const ONE_MINUTE: u64 = 60;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type BaseLiquidityPools: LiquidityPools<Self::AccountId>;
	type PoolManager: MarginProtocolLiquidityPoolsManager;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
	type MaxSwap: Get<Fixed128>;
	type UnixTime: UnixTime;
	type Moment: AtLeast32Bit + Parameter + Default + Copy + From<u64>;
}

decl_storage! {
	trait Store for Module<T: Trait> as MarginLiquidityPools {
		pub TradingPairOptions get(fn trading_pair_options): map hasher(twox_64_concat) TradingPair => MarginTradingPairOption<T::Moment>;
		pub PoolOptions get(fn pool_options): map hasher(twox_64_concat) LiquidityPoolId => MarginPoolOption;
		pub PoolTradingPairOptions: double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) TradingPair => MarginPoolTradingPairOption;
		pub AccumulatedSwapRates get(fn accumulated_swap_rate): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) TradingPair => SwapRate;
		pub DefaultMinLeveragedAmount get(fn default_min_leveraged_amount) config(): Balance;
		pub LastAccumulateTime get(fn last_accumulate_time): T::Moment;
	}

	add_extra_genesis {
		config(margin_liquidity_config): Vec<(TradingPair, Balance, AccumulateConfig<T::Moment>, SwapRate)>;

		build(|config: &GenesisConfig<T>| {
			config.margin_liquidity_config.iter().for_each(|(pair, max_spread, accumulate_config, swap_rate)| {
				<TradingPairOptions<T>>::insert(&pair, MarginTradingPairOption {
					enabled: true,
					swap_rate: swap_rate.clone(),
					max_spread: Some(*max_spread),
					accumulate_config: Some(accumulate_config.clone()),
				});
			})
		})
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::Moment,
	{
		/// Set spread (who, pool_id, pair, bid, ask)
		SetSpread(AccountId, LiquidityPoolId, TradingPair, Balance, Balance),
		/// Set enabled trades (who, pool_id, pair, enabled)
		SetEnabledTrades(AccountId, LiquidityPoolId, TradingPair, Leverages),
		/// Swap rate updated (pair, swap_rate)
		SwapRateUpdated(TradingPair, SwapRate),
		/// Accumulated swap rate updated (pool_id, pair, accumulated_swap_rate)
		AccumulatedSwapRateUpdated(LiquidityPoolId, TradingPair, SwapRate),
		/// Additional swap rate updated (who, pool_id, additional_swap_rate)
		AdditionalSwapRateUpdated(AccountId, LiquidityPoolId, Fixed128),
		/// Max spread updated (pair, spread)
		MaxSpreadUpdated(TradingPair, Balance),
		/// Set accumulate (pair, frequency, offset)
		SetAccumulate(TradingPair, Moment, Moment),
		/// Trading pair enabled (pair)
		TradingPairEnabled(TradingPair),
		/// Trading pair disabled (pair)
		TradingPairDisabled(TradingPair),
		/// LiquidityPool trading pair enabled (pair)
		LiquidityPoolTradingPairEnabled(TradingPair),
		/// LiquidityPool trading pair disabled (pair)
		LiquidityPoolTradingPairDisabled(TradingPair),
		/// Set default min leveraged amount (default_min_leveraged_amount)
		SetDefaultMinLeveragedAmount(Balance),
		/// Set min leveraged amount (pool_id, min_leveraged_amount)
		SetMinLeveragedAmount(LiquidityPoolId, Balance),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = 10_000]
		pub fn set_spread(origin, #[compact] pool_id: LiquidityPoolId, pair: TradingPair, #[compact] bid: Balance, #[compact] ask: Balance) {
			let who = ensure_signed(origin)?;
			Self::_set_spread(&who, pool_id, pair, bid, ask)?;
			Self::deposit_event(RawEvent::SetSpread(who, pool_id, pair, bid, ask));
		}

		#[weight = 10_000]
		pub fn set_enabled_trades(origin, #[compact] pool_id: LiquidityPoolId, pair: TradingPair, enabled: Leverages) {
			let who = ensure_signed(origin)?;
			Self::_set_enabled_trades(&who, pool_id, pair, enabled)?;
			Self::deposit_event(RawEvent::SetEnabledTrades(who, pool_id, pair, enabled));
		}

		#[weight = 10_000]
		pub fn set_swap_rate(origin, pair: TradingPair, rate: SwapRate) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;

			ensure!(rate.long.saturating_abs() <= T::MaxSwap::get(), Error::<T>::SwapRateTooHigh);
			ensure!(rate.short.saturating_abs() <= T::MaxSwap::get(), Error::<T>::SwapRateTooHigh);

			<TradingPairOptions<T>>::mutate(&pair, |o| o.swap_rate = rate.clone());

			Self::deposit_event(RawEvent::SwapRateUpdated(pair, rate));
		}

		#[weight = 10_000]
		pub fn set_additional_swap(origin, #[compact] pool_id: LiquidityPoolId,  rate: Fixed128) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

			PoolOptions::mutate(&pool_id, |o| o.additional_swap_rate = rate);

			Self::deposit_event(RawEvent::AdditionalSwapRateUpdated(who, pool_id, rate));
		}

		#[weight = 10_000]
		pub fn set_max_spread(origin, pair: TradingPair, #[compact] max_spread: Balance) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;

			<TradingPairOptions<T>>::mutate(&pair, |o| o.max_spread = Some(max_spread));

			Self::deposit_event(RawEvent::MaxSpreadUpdated(pair, max_spread));
		}

		#[weight = 10_000]
		pub fn set_accumulate(origin, pair: TradingPair, frequency: T::Moment, offset: T::Moment) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;

			ensure!(frequency >= ONE_MINUTE.into(), Error::<T>::FrequencyTooLow);

			<TradingPairOptions<T>>::mutate(
				&pair,
				|o| o.accumulate_config = Some(AccumulateConfig { frequency, offset })
			);

			Self::deposit_event(RawEvent::SetAccumulate(pair, frequency, offset));
		}

		#[weight = 10_000]
		pub fn enable_trading_pair(origin, pair: TradingPair) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;

			<TradingPairOptions<T>>::mutate(&pair, |o| o.enabled = true);

			Self::deposit_event(RawEvent::TradingPairEnabled(pair))
		}

		#[weight = 10_000]
		pub fn disable_trading_pair(origin, pair: TradingPair) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;

			<TradingPairOptions<T>>::mutate(&pair, |o| o.enabled = false);

			Self::deposit_event(RawEvent::TradingPairDisabled(pair))
		}

		#[weight = 10_000]
		pub fn liquidity_pool_enable_trading_pair(origin, #[compact] pool_id: LiquidityPoolId, pair: TradingPair) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
			ensure!(Self::is_trading_pair_enabled(pair), Error::<T>::TradingPairNotEnabled);

			<T::PoolManager as MarginProtocolLiquidityPoolsManager>::ensure_can_enable_trading_pair(pool_id, pair)?;

			PoolTradingPairOptions::mutate(&pool_id, &pair, |o| o.enabled = true);

			Self::deposit_event(RawEvent::LiquidityPoolTradingPairEnabled(pair))
		}

		#[weight = 10_000]
		pub fn liquidity_pool_disable_trading_pair(origin, #[compact] pool_id: LiquidityPoolId, pair: TradingPair) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

			PoolTradingPairOptions::mutate(&pool_id, &pair, |o| o.enabled = false);

			Self::deposit_event(RawEvent::LiquidityPoolTradingPairDisabled(pair))
		}

		#[weight = 10_000]
		pub fn set_default_min_leveraged_amount(origin, #[compact] amount: Balance) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			DefaultMinLeveragedAmount::put(amount);
			Self::deposit_event(RawEvent::SetDefaultMinLeveragedAmount(amount))
		}

		#[weight = 10_000]
		pub fn set_min_leveraged_amount(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

			PoolOptions::mutate(&pool_id, |o| o.min_leveraged_amount = amount);

			Self::deposit_event(RawEvent::SetMinLeveragedAmount(pool_id, amount))
		}

		fn on_initialize() -> Weight {
			let now_as_mins: T::Moment = (T::UnixTime::now().as_secs() / ONE_MINUTE).into();
			// Truncate seconds, keep minutes
			let now_as_secs: T::Moment = now_as_mins * ONE_MINUTE.into();

			<TradingPairOptions<T>>::iter().for_each(|(pair, option)| {
				if let Some(accumulate_config) = option.accumulate_config {
					let frequency_as_mins = accumulate_config.frequency / ONE_MINUTE.into();
					let offset_as_mins = accumulate_config.offset / ONE_MINUTE.into();

					if now_as_mins > 0.into() && frequency_as_mins > 0.into()
						&& now_as_mins % frequency_as_mins == offset_as_mins
						&& <LastAccumulateTime<T>>::get() != now_as_secs
					{
						<LastAccumulateTime<T>>::set(now_as_secs);
						Self::_accumulate_rates(pair);
					}
				}
			});
			10_000
		}
	}
}

decl_error! {
	// MarginLiquidityPools module errors
	pub enum Error for Module<T: Trait> {
		NoPermission,
		SwapRateTooHigh,
		SwapRateTooLow,
		SpreadTooHigh,
		TradingPairNotEnabled,
		NumOutOfBound,
		FrequencyTooLow,
	}
}

// Storage getters
impl<T: Trait> Module<T> {
	// Trading pair option

	pub fn max_spread(pair: TradingPair) -> Option<Balance> {
		Self::trading_pair_options(pair).max_spread
	}

	pub fn accumulate_config(pair: TradingPair) -> Option<AccumulateConfig<T::Moment>> {
		Self::trading_pair_options(pair).accumulate_config
	}

	pub fn swap_rate(pair: TradingPair) -> SwapRate {
		Self::trading_pair_options(pair).swap_rate
	}

	pub fn is_trading_pair_enabled(pair: TradingPair) -> bool {
		Self::trading_pair_options(pair).enabled
	}

	// Pool margin option

	pub fn additional_swap_rate(pool_id: LiquidityPoolId) -> Fixed128 {
		Self::pool_options(pool_id).additional_swap_rate
	}

	/// Min leveraged amount. `max(min_leveraged_amount, default_min_leveraged_amount)` will be used.
	pub fn min_leveraged_amount(pool_id: LiquidityPoolId) -> Balance {
		let pool_min_leveraged_amount = Self::pool_options(pool_id).min_leveraged_amount;
		max(pool_min_leveraged_amount, Self::default_min_leveraged_amount())
	}

	// Pool trading pair margin option

	/// `PoolTradingPairOptions` getter. Bid/ask spread is capped by max spread.
	pub fn pool_trading_pair_options(pool_id: LiquidityPoolId, pair: TradingPair) -> MarginPoolTradingPairOption {
		let mut option = PoolTradingPairOptions::get(pool_id, pair);
		if let Some(max_spread) = Self::max_spread(pair) {
			option.bid_spread = option.bid_spread.map(|s| s.min(max_spread));
			option.ask_spread = option.ask_spread.map(|s| s.min(max_spread));
		}
		option
	}

	pub fn is_pool_trading_pair_enabled(pool_id: LiquidityPoolId, pair: TradingPair) -> bool {
		PoolTradingPairOptions::get(pool_id, pair).enabled
	}

	pub fn is_pool_trading_pair_leverage_enabled(
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		leverage: Leverage,
	) -> bool {
		Self::pool_trading_pair_options(pool_id, pair)
			.enabled_trades
			.contains(leverage)
	}
}

impl<T: Trait> LiquidityPools<T::AccountId> for Module<T> {
	fn all() -> Vec<LiquidityPoolId> {
		T::BaseLiquidityPools::all()
	}

	fn is_owner(pool_id: LiquidityPoolId, who: &T::AccountId) -> bool {
		T::BaseLiquidityPools::is_owner(pool_id, who)
	}

	/// Check if pool exists
	fn pool_exists(pool_id: LiquidityPoolId) -> bool {
		T::BaseLiquidityPools::pool_exists(pool_id)
	}

	/// Check collateral balance of `pool_id`.
	fn liquidity(pool_id: LiquidityPoolId) -> Balance {
		T::BaseLiquidityPools::liquidity(pool_id)
	}

	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(source: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		T::BaseLiquidityPools::deposit_liquidity(source, pool_id, amount)
	}

	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(dest: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		T::BaseLiquidityPools::withdraw_liquidity(dest, pool_id, amount)
	}
}

impl<T: Trait> MarginProtocolLiquidityPools<T::AccountId> for Module<T> {
	fn is_allowed_position(pool_id: LiquidityPoolId, pair: TradingPair, leverage: Leverage) -> bool {
		Self::is_pool_trading_pair_leverage_enabled(pool_id, pair, leverage)
	}

	fn get_bid_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Balance> {
		Self::pool_trading_pair_options(pool_id, pair).bid_spread
	}

	fn get_ask_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Balance> {
		Self::pool_trading_pair_options(pool_id, pair).ask_spread
	}

	fn get_swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> Fixed128 {
		let max_swap = T::MaxSwap::get();
		let swap_rate = Self::swap_rate(pair);
		let additional_swap_rate = Self::additional_swap_rate(pool_id);

		let swap_rate = if is_long { swap_rate.long } else { swap_rate.short };
		// adjust_swap = swap - abs(swap) * additional_swap_rate
		let adjust_swap = swap_rate.saturating_sub(swap_rate.saturating_abs().saturating_mul(additional_swap_rate));

		if adjust_swap.saturating_abs() <= max_swap {
			adjust_swap
		} else {
			if adjust_swap.is_positive() {
				max_swap
			} else {
				fixed_128_mul_signum(max_swap, -1)
			}
		}
	}

	fn get_accumulated_swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> Fixed128 {
		let accumulated_swap_rate = Self::accumulated_swap_rate(pool_id, pair);
		if is_long {
			accumulated_swap_rate.long
		} else {
			accumulated_swap_rate.short
		}
	}

	fn can_open_position(
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		leverage: Leverage,
		leveraged_amount: Balance,
	) -> bool {
		Self::is_pool_trading_pair_leverage_enabled(pool_id, pair, leverage)
			&& Self::is_trading_pair_enabled(pair)
			&& Self::is_pool_trading_pair_enabled(pool_id, pair)
			&& leveraged_amount >= Self::min_leveraged_amount(pool_id)
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _set_spread(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		bid: Balance,
		ask: Balance,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		PoolTradingPairOptions::mutate(pool_id, pair, |o| {
			o.bid_spread = Some(bid);
			o.ask_spread = Some(ask);
		});
		Ok(())
	}

	fn _set_enabled_trades(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		enabled: Leverages,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		PoolTradingPairOptions::mutate(pool_id, pair, |o| o.enabled_trades = enabled);
		Ok(())
	}

	fn _accumulate_rates(pair: TradingPair) {
		for pool_id in T::BaseLiquidityPools::all() {
			let long_rate = Self::get_swap_rate(pool_id, pair, true);
			let short_rate = Self::get_swap_rate(pool_id, pair, false);

			let mut accumulated = Self::accumulated_swap_rate(pool_id, pair);
			accumulated.long = accumulated.long.saturating_add(long_rate);
			accumulated.short = accumulated.short.saturating_add(short_rate);
			AccumulatedSwapRates::insert(pool_id, pair, accumulated.clone());

			Self::deposit_event(RawEvent::AccumulatedSwapRateUpdated(pool_id, pair, accumulated))
		}
	}
}

impl<T: Trait> OnDisableLiquidityPool for Module<T> {
	fn on_disable(pool_id: LiquidityPoolId) {
		PoolTradingPairOptions::remove_prefix(&pool_id);
	}
}

impl<T: Trait> OnRemoveLiquidityPool for Module<T> {
	fn on_remove(pool_id: LiquidityPoolId) {
		PoolTradingPairOptions::remove_prefix(&pool_id);
		AccumulatedSwapRates::remove_prefix(&pool_id);
		PoolOptions::remove(&pool_id);
	}
}
