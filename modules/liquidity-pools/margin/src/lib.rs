#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	storage::IterableStorageMap,
	traits::{EnsureOrigin, Get, UnixTime},
	weights::Weight,
	Parameter,
};
use frame_system::ensure_signed;
use primitives::{
	arithmetic::fixed_i128_mul_signum, AccumulateConfig, Balance, Leverage, Leverages, LiquidityPoolId, Price,
	SwapRate, TradingPair,
};
use sp_arithmetic::{FixedI128, FixedPointNumber};
use sp_runtime::{
	traits::{AtLeast32Bit, Saturating, Zero},
	DispatchResult, ModuleId, RuntimeDebug,
};
use sp_std::{cmp::max, prelude::*, result};

use orml_utilities::with_transaction_result;

use traits::{
	LiquidityPools, MarginProtocolLiquidityPools, MarginProtocolLiquidityPoolsManager, OnDisableLiquidityPool,
	OnRemoveLiquidityPool, OpenPositionError,
};

mod default_weight;
mod mock;
mod tests;

pub trait WeightInfo {
	fn set_spread() -> Weight;
	fn set_enabled_leverages() -> Weight;
	fn set_swap_rate() -> Weight;
	fn set_additional_swap_rate() -> Weight;
	fn set_max_spread() -> Weight;
	fn set_accumulate_config() -> Weight;
	fn enable_trading_pair() -> Weight;
	fn disable_trading_pair() -> Weight;
	fn liquidity_pool_enable_trading_pair() -> Weight;
	fn liquidity_pool_disable_trading_pair() -> Weight;
	fn set_default_min_leveraged_amount() -> Weight;
	fn set_min_leveraged_amount() -> Weight;
	fn on_initialize(r: u32, w: u32) -> Weight;
}

/// Trading pair option of margin liquidity pools.
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MarginTradingPairOption<Moment> {
	/// Is enabled for trading.
	///
	/// DEFAULT-NOTE: default not enabled.
	pub enabled: bool,

	/// The max spread. The minimum of max spread and pool's spread would be used in trading.
	pub max_spread: Option<Price>,

	/// Swap rate.
	///
	/// DEFAULT-NOTE: zero rate if not set.
	pub swap_rate: SwapRate,

	/// The accumulate config.
	pub accumulate_config: Option<AccumulateConfig<Moment>>,
}

/// Pool option of margin liquidity pools.
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MarginPoolOption {
	/// Additional swap rate, to adjust the swap rate in `MarginTradingPairOption`.
	///
	/// DEFAULT-NOTE: no adjustment for this pool.
	pub additional_swap_rate: FixedI128,

	/// Min leveraged amount to open a position. If not set or smaller than default min leveraged
	/// amount, the default value will be used instead.
	///
	/// DEFAULT-NOTE: no min requirement for this pool.
	pub min_leveraged_amount: Balance,
}

/// Trading pair option in a pool of margin.
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MarginPoolTradingPairOption {
	/// Is enabled in pool.
	///
	/// DEFAULT-NOTE: default not enabled.
	pub enabled: bool,

	/// Bid spread.
	///
	/// DEFAULT-NOTE: `None`, pool owner must set spread.
	pub bid_spread: Option<Price>,

	/// Ask spread
	///
	/// DEFAULT-NOTE: `None`, pool owner must set spread.
	pub ask_spread: Option<Price>,

	/// Enabled leverages.
	///
	/// DEFAULT-NOTE: No leverage.
	pub enabled_trades: Leverages,
}

pub const MODULE_ID: ModuleId = ModuleId(*b"lami/mlp");
pub const ONE_MINUTE: u64 = 60;

pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// The `LiquidityPools` implementation.
	type BaseLiquidityPools: LiquidityPools<Self::AccountId>;

	/// The `MarginProtocolLiquidityPoolsManager` implementation.
	type PoolManager: MarginProtocolLiquidityPoolsManager;

	/// Required origin for updating protocol options.
	type UpdateOrigin: EnsureOrigin<Self::Origin>;

	/// Maximum swap rate.
	type MaxSwapRate: Get<FixedI128>;

	/// Time used for computing accumulated swap rate.
	type UnixTime: UnixTime;

	/// Type used for expressing timestamp.
	type Moment: AtLeast32Bit + Parameter + Default + Copy + From<u64>;

	/// Weight information for the extrinsics in this module.
	type WeightInfo: WeightInfo;
}

decl_storage! {
	trait Store for Module<T: Config> as MarginLiquidityPools {
		/// Trading pair options.
		pub TradingPairOptions get(fn trading_pair_options): map hasher(twox_64_concat) TradingPair => MarginTradingPairOption<T::Moment>;

		/// Liquidity pool options, managed by pool owner.
		pub PoolOptions get(fn pool_options): map hasher(twox_64_concat) LiquidityPoolId => MarginPoolOption;

		/// Trading pair options in a liquidity pool.
		///
		/// Getter is implemented manually to cap the spread with max spread.
		pub PoolTradingPairOptions: double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) TradingPair => MarginPoolTradingPairOption;

		/// The accumulated swap rate of trading pairs in liquidity pools.
		pub AccumulatedSwapRates get(fn accumulated_swap_rate): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) TradingPair => SwapRate;

		/// The default minimum leveraged amount allowed to open a position.
		pub DefaultMinLeveragedAmount get(fn default_min_leveraged_amount) config(): Balance;

		/// The last time swap rate was accumulated.
		pub LastAccumulateTime get(fn last_accumulate_time): T::Moment;
	}

	add_extra_genesis {
		config(margin_liquidity_config): Vec<(TradingPair, Price, AccumulateConfig<T::Moment>, SwapRate)>;

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
		<T as frame_system::Config>::AccountId,
		<T as Config>::Moment,
	{
		/// Spread set: \[who, pool_id, pair, bid, ask\]
		SpreadSet(AccountId, LiquidityPoolId, TradingPair, Price, Price),

		/// Enabled trades set: \[who, pool_id, pair, enabled_leverages\]
		EnabledTradesSet(AccountId, LiquidityPoolId, TradingPair, Leverages),

		/// Swap rate updated: \[pair, swap_rate\]
		SwapRateUpdated(TradingPair, SwapRate),

		/// Accumulated swap rate updated: \[pool_id, pair, accumulated_swap_rate\]
		AccumulatedSwapRateUpdated(LiquidityPoolId, TradingPair, SwapRate),

		/// Additional swap rate updated: \[who, pool_id, additional_swap_rate\]
		AdditionalSwapRateUpdated(AccountId, LiquidityPoolId, FixedI128),

		/// Max spread updated: \[pair, spread\]
		MaxSpreadUpdated(TradingPair, Price),

		/// Accumulate set: \[pair, frequency, offset\]
		AccumulateConfigSet(TradingPair, Moment, Moment),

		/// Trading pair enabled: \[pair\]
		TradingPairEnabled(TradingPair),

		/// Trading pair disabled: \[pair\]
		TradingPairDisabled(TradingPair),

		/// Trading pair enabled in pool: \[pool_id, pair\]
		LiquidityPoolTradingPairEnabled(LiquidityPoolId, TradingPair),

		/// Trading pair disabled in pool: \[pool_id, pair\]
		LiquidityPoolTradingPairDisabled(LiquidityPoolId, TradingPair),

		/// Default min leveraged amount set: \[default_min_leveraged_amount\]
		DefaultMinLeveragedAmountSet(Balance),

		/// Pool min leveraged amount set: \[pool_id, min_leveraged_amount\]
		MinLeveragedAmountSet(LiquidityPoolId, Balance),
	}
);

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Set bid and ask spread for `pair` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::set_spread()]
		pub fn set_spread(origin, #[compact] pool_id: LiquidityPoolId, pair: TradingPair, #[compact] bid: Price, #[compact] ask: Price) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_set_spread(&who, pool_id, pair, bid, ask)?;
				Self::deposit_event(RawEvent::SpreadSet(who, pool_id, pair, bid, ask));
				Ok(())
			})?;
		}

		/// Set enabled leverages for `pair` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::set_enabled_leverages()]
		pub fn set_enabled_leverages(origin, #[compact] pool_id: LiquidityPoolId, pair: TradingPair, enabled: Leverages) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_set_enabled_leverages(&who, pool_id, pair, enabled)?;
				Self::deposit_event(RawEvent::EnabledTradesSet(who, pool_id, pair, enabled));
				Ok(())
			})?;
		}

		/// Set swap rate for `pair`.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_swap_rate()]
		pub fn set_swap_rate(origin, pair: TradingPair, rate: SwapRate) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;

				ensure!(rate.long.saturating_abs() <= T::MaxSwapRate::get(), Error::<T>::SwapRateTooHigh);
				ensure!(rate.short.saturating_abs() <= T::MaxSwapRate::get(), Error::<T>::SwapRateTooHigh);

				<TradingPairOptions<T>>::mutate(&pair, |o| o.swap_rate = rate.clone());

				Self::deposit_event(RawEvent::SwapRateUpdated(pair, rate));

				Ok(())
			})?;
		}

		/// Set additional swap rate for `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::set_additional_swap_rate()]
		pub fn set_additional_swap_rate(origin, #[compact] pool_id: LiquidityPoolId, rate: FixedI128) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

				PoolOptions::mutate(&pool_id, |o| o.additional_swap_rate = rate);

				Self::deposit_event(RawEvent::AdditionalSwapRateUpdated(who, pool_id, rate));

				Ok(())
			})?;
		}

		/// Set maximum spread for `pair`.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_max_spread()]
		pub fn set_max_spread(origin, pair: TradingPair, #[compact] max_spread: Price) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				<TradingPairOptions<T>>::mutate(&pair, |o| o.max_spread = Some(max_spread));
				Self::deposit_event(RawEvent::MaxSpreadUpdated(pair, max_spread));
				Ok(())
			})?;
		}

		/// Set swap rate accumulation configuration.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_accumulate_config()]
		pub fn set_accumulate_config(origin, pair: TradingPair, frequency: T::Moment, offset: T::Moment) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;

				ensure!(frequency >= ONE_MINUTE.into(), Error::<T>::FrequencyTooLow);

				<TradingPairOptions<T>>::mutate(
					&pair,
					|o| o.accumulate_config = Some(AccumulateConfig { frequency, offset })
				);

				Self::deposit_event(RawEvent::AccumulateConfigSet(pair, frequency, offset));

				Ok(())
			})?;
		}

		/// Enable a trading pair.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::enable_trading_pair()]
		pub fn enable_trading_pair(origin, pair: TradingPair) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				<TradingPairOptions<T>>::mutate(&pair, |o| o.enabled = true);
				Self::deposit_event(RawEvent::TradingPairEnabled(pair));
				Ok(())
			})?;
		}

		/// Disable a trading pair.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::disable_trading_pair()]
		pub fn disable_trading_pair(origin, pair: TradingPair) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				<TradingPairOptions<T>>::mutate(&pair, |o| o.enabled = false);
				Self::deposit_event(RawEvent::TradingPairDisabled(pair));
				Ok(())
			})?;
		}

		/// Enable `pair` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::liquidity_pool_enable_trading_pair()]
		pub fn liquidity_pool_enable_trading_pair(origin, #[compact] pool_id: LiquidityPoolId, pair: TradingPair) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
				ensure!(Self::is_trading_pair_enabled(pair), Error::<T>::TradingPairNotEnabled);

				<T::PoolManager as MarginProtocolLiquidityPoolsManager>::ensure_can_enable_trading_pair(pool_id, pair)?;

				PoolTradingPairOptions::mutate(&pool_id, &pair, |o| o.enabled = true);

				Self::deposit_event(RawEvent::LiquidityPoolTradingPairEnabled(pool_id, pair));

				Ok(())
			})?;
		}

		/// Disable `pair` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::liquidity_pool_disable_trading_pair()]
		pub fn liquidity_pool_disable_trading_pair(origin, #[compact] pool_id: LiquidityPoolId, pair: TradingPair) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

				PoolTradingPairOptions::mutate(&pool_id, &pair, |o| o.enabled = false);

				Self::deposit_event(RawEvent::LiquidityPoolTradingPairDisabled(pool_id, pair));

				Ok(())
			})?;
		}

		/// Set default minimum leveraged amount to open a position.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_default_min_leveraged_amount()]
		pub fn set_default_min_leveraged_amount(origin, #[compact] amount: Balance) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				DefaultMinLeveragedAmount::put(amount);
				Self::deposit_event(RawEvent::DefaultMinLeveragedAmountSet(amount));
				Ok(())
			})?;
		}

		/// Set minimum leveraged amount to open a position in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::set_min_leveraged_amount()]
		pub fn set_min_leveraged_amount(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

				PoolOptions::mutate(&pool_id, |o| o.min_leveraged_amount = amount);

				Self::deposit_event(RawEvent::MinLeveragedAmountSet(pool_id, amount));

				Ok(())
			})?;
		}

		fn on_initialize() -> Weight {
			let now_as_mins: T::Moment = (T::UnixTime::now().as_secs() / ONE_MINUTE).into();
			// Truncate seconds, keep minutes
			let now_as_secs: T::Moment = now_as_mins * ONE_MINUTE.into();

			let mut read_count = 0;
			let mut write_count = 0;
			<TradingPairOptions<T>>::iter().for_each(|(pair, option)| {
				if let Some(accumulate_config) = option.accumulate_config {
					read_count += 1;
					let frequency_as_mins = accumulate_config.frequency / ONE_MINUTE.into();
					let offset_as_mins = accumulate_config.offset / ONE_MINUTE.into();

					if now_as_mins > Zero::zero() && frequency_as_mins > Zero::zero()
						&& now_as_mins % frequency_as_mins == offset_as_mins
						&& <LastAccumulateTime<T>>::get() != now_as_secs
					{
						write_count += 1;
						<LastAccumulateTime<T>>::set(now_as_secs);
						Self::accumulate_rates(pair);
					}
				}
			});
			T::WeightInfo::on_initialize(read_count, write_count)
		}
	}
}

decl_error! {
	/// Errors for the margin liquidity pools module.
	pub enum Error for Module<T: Config> {
		/// Caller doesn't have permission.
		NoPermission,

		/// Swap rate is higher than max allowed.
		SwapRateTooHigh,

		/// Trading pair is not enabled in pool.
		TradingPairNotEnabled,

		/// `frequency` of accumulate config is too low.
		FrequencyTooLow,
	}
}

// Storage getters
impl<T: Config> Module<T> {
	// Trading pair option

	pub fn max_spread(pair: TradingPair) -> Option<Price> {
		Self::trading_pair_options(pair).max_spread
	}

	pub fn accumulate_config(pair: TradingPair) -> Option<AccumulateConfig<T::Moment>> {
		Self::trading_pair_options(pair).accumulate_config
	}

	pub fn swap_rate_of_pair(pair: TradingPair) -> SwapRate {
		Self::trading_pair_options(pair).swap_rate
	}

	pub fn is_trading_pair_enabled(pair: TradingPair) -> bool {
		Self::trading_pair_options(pair).enabled
	}

	// Pool margin option

	pub fn additional_swap_rate(pool_id: LiquidityPoolId) -> FixedI128 {
		Self::pool_options(pool_id).additional_swap_rate
	}

	/// Min leveraged amount. `max(min_leveraged_amount, default_min_leveraged_amount)` will be
	/// used.
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

impl<T: Config> LiquidityPools<T::AccountId> for Module<T> {
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

impl<T: Config> MarginProtocolLiquidityPools<T::AccountId> for Module<T> {
	fn bid_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Price> {
		Self::pool_trading_pair_options(pool_id, pair).bid_spread
	}

	fn ask_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Price> {
		Self::pool_trading_pair_options(pool_id, pair).ask_spread
	}

	fn swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> FixedI128 {
		let max_swap = T::MaxSwapRate::get();
		let swap_rate = Self::swap_rate_of_pair(pair);
		let additional_swap_rate = Self::additional_swap_rate(pool_id);

		let swap_rate = if is_long { swap_rate.long } else { swap_rate.short };
		// adjust_swap = swap - abs(swap) * additional_swap_rate
		let adjust_swap = swap_rate.saturating_sub(swap_rate.saturating_abs().saturating_mul(additional_swap_rate));

		if adjust_swap.saturating_abs() <= max_swap {
			adjust_swap
		} else if adjust_swap.is_positive() {
			max_swap
		} else {
			fixed_i128_mul_signum(max_swap, -1)
		}
	}

	fn accumulated_swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> FixedI128 {
		let accumulated_swap_rate = Self::accumulated_swap_rate(pool_id, pair);
		if is_long {
			accumulated_swap_rate.long
		} else {
			accumulated_swap_rate.short
		}
	}

	fn ensure_can_open_position(
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		leverage: Leverage,
		leveraged_amount: Balance,
	) -> result::Result<(), OpenPositionError> {
		if !Self::is_pool_trading_pair_leverage_enabled(pool_id, pair, leverage) {
			return Err(OpenPositionError::LeverageNotAllowedInPool);
		}
		if !Self::is_trading_pair_enabled(pair) {
			return Err(OpenPositionError::TradingPairNotEnabled);
		}
		if !Self::is_pool_trading_pair_enabled(pool_id, pair) {
			return Err(OpenPositionError::TradingPairNotEnabledInPool);
		}
		if leveraged_amount < Self::min_leveraged_amount(pool_id) {
			return Err(OpenPositionError::BelowMinLeveragedAmount);
		}
		Ok(())
	}
}

// Dispatchable calls implementation
impl<T: Config> Module<T> {
	fn do_set_spread(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		bid: Price,
		ask: Price,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		PoolTradingPairOptions::mutate(pool_id, pair, |o| {
			o.bid_spread = Some(bid);
			o.ask_spread = Some(ask);
		});
		Ok(())
	}

	fn do_set_enabled_leverages(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		enabled: Leverages,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		PoolTradingPairOptions::mutate(pool_id, pair, |o| o.enabled_trades = enabled);
		Ok(())
	}
}

// Private methods
impl<T: Config> Module<T> {
	fn accumulate_rates(pair: TradingPair) {
		for pool_id in T::BaseLiquidityPools::all() {
			let long_rate = Self::swap_rate(pool_id, pair, true);
			let short_rate = Self::swap_rate(pool_id, pair, false);

			let mut accumulated = Self::accumulated_swap_rate(pool_id, pair);
			accumulated.long = accumulated.long.saturating_add(long_rate);
			accumulated.short = accumulated.short.saturating_add(short_rate);
			AccumulatedSwapRates::insert(pool_id, pair, accumulated.clone());

			Self::deposit_event(RawEvent::AccumulatedSwapRateUpdated(pool_id, pair, accumulated))
		}
	}
}

impl<T: Config> OnDisableLiquidityPool for Module<T> {
	fn on_disable(pool_id: LiquidityPoolId) {
		PoolTradingPairOptions::remove_prefix(&pool_id);
	}
}

impl<T: Config> OnRemoveLiquidityPool for Module<T> {
	fn on_remove(pool_id: LiquidityPoolId) {
		PoolTradingPairOptions::remove_prefix(&pool_id);
		AccumulatedSwapRates::remove_prefix(&pool_id);
		PoolOptions::remove(&pool_id);
	}
}
