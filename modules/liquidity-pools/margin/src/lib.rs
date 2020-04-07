#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageMap, traits::Get,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use orml_traits::{BasicCurrency, MultiCurrency};
use orml_utilities::Fixed128;
use primitives::{AccumulateConfig, Balance, CurrencyId, Leverage, Leverages, LiquidityPoolId, TradingPair};
use sp_runtime::{
	traits::{AccountIdConversion, EnsureOrigin, One, Saturating},
	DispatchResult, ModuleId, PerThing, Permill, RuntimeDebug,
};
use sp_std::cmp::max;
use sp_std::{prelude::*, result};
use traits::{
	LiquidityPoolManager, LiquidityPools, MarginProtocolLiquidityPools, OnDisableLiquidityPool, OnRemoveLiquidityPool,
};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MarginLiquidityPoolOption {
	pub bid_spread: Permill,
	pub ask_spread: Permill,
	pub enabled_trades: Leverages,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct SwapRate {
	pub long: Fixed128,
	pub short: Fixed128,
}

const MODULE_ID: ModuleId = ModuleId(*b"lami/mlp");

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Balance>;
	type PoolManager: LiquidityPoolManager<LiquidityPoolId, Balance>;
	type ExistentialDeposit: Get<Balance>;
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
	type MaxSwap: Get<Fixed128>;
}

decl_storage! {
	trait Store for Module<T: Trait> as MarginLiquidityPools {
		pub NextPoolId get(fn next_pool_id): LiquidityPoolId;
		pub Owners get(fn owners): map hasher(blake2_128_concat) LiquidityPoolId => Option<(T::AccountId, LiquidityPoolId)>;
		pub LiquidityPoolOptions get(fn liquidity_pool_options): double_map hasher(blake2_128_concat) LiquidityPoolId, hasher(blake2_128_concat) TradingPair => Option<MarginLiquidityPoolOption>;
		pub Balances get(fn balances): map hasher(blake2_128_concat) LiquidityPoolId => Balance;
		pub SwapRates get(fn swap_rate): map hasher(blake2_128_concat) TradingPair => Option<SwapRate>;
		pub AccumulatedSwapRates get(fn accumulated_swap_rate): double_map hasher(blake2_128_concat) LiquidityPoolId, hasher(blake2_128_concat) TradingPair => SwapRate;
		pub AdditionalSwapRate get(fn additional_swap_rate): map hasher(blake2_128_concat) LiquidityPoolId => Option<Fixed128>;
		pub MaxSpread get(fn max_spread): map hasher(blake2_128_concat) TradingPair => Permill;
		pub Accumulates get(fn accumulate): map hasher(blake2_128_concat) TradingPair => Option<(AccumulateConfig<T::BlockNumber>, TradingPair)>;
		pub EnabledTradingPairs get(fn enabled_trading_pair): map hasher(blake2_128_concat) TradingPair => Option<TradingPair>;
		pub LiquidityPoolEnabledTradingPairs get(fn liquidity_pool_enabled_trading_pair): double_map hasher(blake2_128_concat) LiquidityPoolId, hasher(blake2_128_concat) TradingPair => Option<TradingPair>;
		pub DefaultMinLeveragedAmount get(fn default_min_leveraged_amount) config(): Balance;
		pub MinLeveragedAmount get(fn min_leveraged_amount): map hasher(blake2_128_concat) LiquidityPoolId => Option<Balance>;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as system::Trait>::BlockNumber,
	{
		/// Liquidity pool created (who, pool_id)
		LiquidityPoolCreated(AccountId, LiquidityPoolId),
		/// Liquidity pool disabled (who, pool_id)
		LiquidityPoolDisabled(AccountId, LiquidityPoolId),
		/// Liquidity pool removed (who, pool_id)
		LiquidityPoolRemoved(AccountId, LiquidityPoolId),
		/// Deposit liquidity (who, pool_id, amount)
		DepositLiquidity(AccountId, LiquidityPoolId, Balance),
		/// Withdraw liquidity (who, pool_id, amount)
		WithdrawLiquidity(AccountId, LiquidityPoolId, Balance),
		/// Set spread (who, pool_id, pair, bid, ask)
		SetSpread(AccountId, LiquidityPoolId, TradingPair, Permill, Permill),
		/// Set enabled trades (who, pool_id, pair, enabled)
		SetEnabledTrades(AccountId, LiquidityPoolId, TradingPair, Leverages),
		/// Swap rate updated (pair, swap_rate)
		SwapRateUpdated(TradingPair, SwapRate),
		/// Accumulated swap rate updated (pool_id, pair, accumulated_swap_rate)
		AccumulatedSwapRateUpdated(LiquidityPoolId, TradingPair, SwapRate),
		/// Additional swap rate updated (who, pool_id, additional_swap_rate)
		AdditionalSwapRateUpdated(AccountId, LiquidityPoolId, Fixed128),
		/// Max spread updated (pair, spread)
		MaxSpreadUpdated(TradingPair, Permill),
		/// Set accumulate (pair, frequency, offset)
		SetAccumulate(TradingPair, BlockNumber, BlockNumber),
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

		const ExistentialDeposit: Balance = T::ExistentialDeposit::get();

		pub fn create_pool(origin) {
			let who = ensure_signed(origin)?;
			let pool_id = Self::_create_pool(&who)?;
			Self::deposit_event(RawEvent::LiquidityPoolCreated(who, pool_id));
		}

		pub fn disable_pool(origin, pool_id: LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_disable_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolDisabled(who, pool_id));
		}

		pub fn remove_pool(origin, pool_id: LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_remove_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolRemoved(who, pool_id));
		}

		pub fn deposit_liquidity(origin, pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_deposit_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::DepositLiquidity(who, pool_id, amount));
		}

		pub fn withdraw_liquidity(origin, pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

			let new_balance = Self::balances(&pool_id).checked_sub(amount).ok_or(Error::<T>::CannotWithdrawAmount)?;
			// check minimum balance
			if new_balance < T::ExistentialDeposit::get() {
				return Err(Error::<T>::CannotWithdrawExistentialDeposit.into());
			}

			Self::_withdraw_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::WithdrawLiquidity(who, pool_id, amount));
		}

		pub fn set_spread(origin, pool_id: LiquidityPoolId, pair: TradingPair, bid: Permill, ask: Permill) {
			let who = ensure_signed(origin)?;
			Self::_set_spread(&who, pool_id, pair, bid, ask)?;
			Self::deposit_event(RawEvent::SetSpread(who, pool_id, pair, bid, ask));
		}

		pub fn set_enabled_trades(origin, pool_id: LiquidityPoolId, pair: TradingPair, enabled: Leverages) {
			let who = ensure_signed(origin)?;
			Self::_set_enabled_trades(&who, pool_id, pair, enabled)?;
			Self::deposit_event(RawEvent::SetEnabledTrades(who, pool_id, pair, enabled));
		}

		pub fn set_swap_rate(origin, pair: TradingPair, rate: SwapRate) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;

			ensure!(rate.long.saturating_abs() <= T::MaxSwap::get(), Error::<T>::SwapRateTooHigh);
			ensure!(rate.short.saturating_abs() <= T::MaxSwap::get(), Error::<T>::SwapRateTooHigh);
			SwapRates::insert(pair, rate.clone());
			Self::deposit_event(RawEvent::SwapRateUpdated(pair, rate));
		}

		pub fn set_additional_swap(origin, pool_id: LiquidityPoolId, rate: Fixed128) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

			ensure!(rate.saturating_abs() <= T::MaxSwap::get(), Error::<T>::SwapRateTooHigh);

			AdditionalSwapRate::insert(pool_id, rate);
			Self::deposit_event(RawEvent::AdditionalSwapRateUpdated(who, pool_id, rate));
		}

		pub fn set_max_spread(origin, pair: TradingPair, max_spread: Permill) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			MaxSpread::insert(pair, max_spread);
			Self::deposit_event(RawEvent::MaxSpreadUpdated(pair, max_spread));
		}

		pub fn set_accumulate(origin, pair: TradingPair, frequency: T::BlockNumber, offset: T::BlockNumber) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			let accumulate = AccumulateConfig { frequency, offset };
			<Accumulates<T>>::insert(pair, (accumulate, pair));
			Self::deposit_event(RawEvent::SetAccumulate(pair, frequency, offset));
		}

		pub fn enable_trading_pair(origin, pair: TradingPair) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			EnabledTradingPairs::insert(&pair, &pair);
			Self::deposit_event(RawEvent::TradingPairEnabled(pair))
		}

		pub fn disable_trading_pair(origin, pair: TradingPair) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			EnabledTradingPairs::remove(&pair);
			Self::deposit_event(RawEvent::TradingPairDisabled(pair))
		}

		pub fn liquidity_pool_enable_trading_pair(origin, pool_id: LiquidityPoolId, pair: TradingPair) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
			ensure!(Self::enabled_trading_pair(&pair).is_some(), Error::<T>::TradingPairNotEnabled);
			LiquidityPoolEnabledTradingPairs::insert(&pool_id, &pair, &pair);
			Self::deposit_event(RawEvent::LiquidityPoolTradingPairEnabled(pair))
		}

		pub fn liquidity_pool_disable_trading_pair(origin, pool_id: LiquidityPoolId, pair: TradingPair) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
			LiquidityPoolEnabledTradingPairs::remove(&pool_id, &pair);
			Self::deposit_event(RawEvent::LiquidityPoolTradingPairDisabled(pair))
		}

		pub fn set_default_min_leveraged_amount(origin, #[compact] amount: Balance) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			DefaultMinLeveragedAmount::put(amount);
			Self::deposit_event(RawEvent::SetDefaultMinLeveragedAmount(amount))
		}

		pub fn set_min_leveraged_amount(origin, pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
			MinLeveragedAmount::insert(pool_id, amount);
			Self::deposit_event(RawEvent::SetMinLeveragedAmount(pool_id, amount))
		}

		fn on_initialize(n: T::BlockNumber) {
			for (_, (accumulate_config, pair)) in <Accumulates<T>>::iter() {
				if n % accumulate_config.frequency == accumulate_config.offset {
					Self::_accumulate_rates(pair);
				}
			}
		}
	}
}

decl_error! {
	// MarginLiquidityPools module errors
	pub enum Error for Module<T: Trait> {
		NoPermission,
		CannotCreateMorePool,
		CannotRemovePool,
		CannotDepositAmount,
		CannotWithdrawAmount,
		CannotWithdrawExistentialDeposit,
		SwapRateTooHigh,
		SwapRateTooLow,
		SpreadTooHigh,
		PoolNotFound,
		TradingPairNotEnabled,
		NumOutOfBound,
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn is_owner(pool_id: LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::owners(pool_id).map_or(false, |(id, _)| &id == who)
	}

	pub fn is_enabled(pool_id: LiquidityPoolId, pair: TradingPair, leverage: Leverage) -> bool {
		Self::liquidity_pool_options(&pool_id, &pair).map_or(false, |pool| pool.enabled_trades.contains(leverage))
	}

	pub fn get_min_leveraged_amount(pool_id: LiquidityPoolId) -> Balance {
		let min_leveraged_amount = Self::min_leveraged_amount(pool_id).unwrap_or(0);
		max(min_leveraged_amount, Self::default_min_leveraged_amount())
	}
}

impl<T: Trait> LiquidityPools<T::AccountId> for Module<T> {
	fn is_owner(pool_id: LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::is_owner(pool_id, who)
	}

	/// Check collateral balance of `pool_id`.
	fn liquidity(pool_id: LiquidityPoolId) -> Balance {
		Self::balances(&pool_id)
	}

	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(source: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		Self::_deposit_liquidity(source, pool_id, amount)
	}

	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(dest: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		Self::_withdraw_liquidity(dest, pool_id, amount)
	}
}

impl<T: Trait> MarginProtocolLiquidityPools<T::AccountId> for Module<T> {
	fn is_allowed_position(pool_id: LiquidityPoolId, pair: TradingPair, leverage: Leverage) -> bool {
		Self::is_enabled(pool_id, pair, leverage)
	}

	fn get_bid_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &pair).map(|pool| pool.bid_spread)
	}

	fn get_ask_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &pair).map(|pool| pool.ask_spread)
	}

	fn get_swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> Fixed128 {
		let max_swap = T::MaxSwap::get();
		let swap_rate = Self::swap_rate(pair).unwrap_or_default();
		let adjust_rate: Fixed128;
		if is_long {
			adjust_rate = swap_rate
				.long
				.saturating_sub(Self::additional_swap_rate(pool_id).unwrap_or_default());
		} else {
			adjust_rate = swap_rate
				.short
				.saturating_sub(Self::additional_swap_rate(pool_id).unwrap_or_default());
		}
		if adjust_rate.saturating_abs() <= max_swap {
			adjust_rate
		} else {
			max_swap
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
		Self::is_enabled(pool_id, pair, leverage)
			&& Self::enabled_trading_pair(&pair).is_some()
			&& Self::liquidity_pool_enabled_trading_pair(&pool_id, &pair).is_some()
			&& leveraged_amount >= Self::get_min_leveraged_amount(pool_id)
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _create_pool(who: &T::AccountId) -> result::Result<LiquidityPoolId, Error<T>> {
		let pool_id = Self::next_pool_id();
		// increment next pool id
		let next_pool_id = pool_id
			.checked_add(One::one())
			.ok_or(Error::<T>::CannotCreateMorePool)?;
		NextPoolId::put(next_pool_id);
		// owner reference
		<Owners<T>>::insert(&pool_id, (who, pool_id));
		Ok(pool_id)
	}

	fn _disable_pool(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		LiquidityPoolOptions::remove_prefix(&pool_id);
		Ok(())
	}

	fn _remove_pool(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		ensure!(T::PoolManager::can_remove(pool_id), Error::<T>::CannotRemovePool);

		let balance = Self::balances(&pool_id);
		// transfer balance to pool owner
		T::LiquidityCurrency::transfer(&Self::account_id(), who, balance)?;

		Balances::remove(&pool_id);
		<Owners<T>>::remove(&pool_id);
		LiquidityPoolOptions::remove_prefix(&pool_id);
		AccumulatedSwapRates::remove_prefix(&pool_id);
		AdditionalSwapRate::remove(&pool_id);
		LiquidityPoolEnabledTradingPairs::remove_prefix(&pool_id);
		MinLeveragedAmount::remove(pool_id);

		Ok(())
	}

	fn _deposit_liquidity(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(<Owners<T>>::contains_key(&pool_id), Error::<T>::PoolNotFound);
		let balance = Self::balances(&pool_id);
		let new_balance = balance.checked_add(amount).ok_or(Error::<T>::CannotDepositAmount)?;
		// transfer amount to this pool
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount)?;
		// update balance
		Balances::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _withdraw_liquidity(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(<Owners<T>>::contains_key(&pool_id), Error::<T>::PoolNotFound);

		let new_balance = Self::balances(&pool_id)
			.checked_sub(amount)
			.ok_or(Error::<T>::CannotWithdrawAmount)?;

		// transfer amount to account
		T::LiquidityCurrency::transfer(&Self::account_id(), who, amount)?;

		// update balance
		Balances::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _set_spread(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		bid: Permill,
		ask: Permill,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let max_spread = Self::max_spread(pair);
		if !max_spread.is_zero() {
			ensure!(ask <= max_spread && bid <= max_spread, Error::<T>::SpreadTooHigh);
		}
		let mut pool = Self::liquidity_pool_options(&pool_id, &pair).unwrap_or_default();
		pool.bid_spread = bid;
		pool.ask_spread = ask;
		LiquidityPoolOptions::insert(&pool_id, &pair, pool);
		Ok(())
	}

	fn _set_enabled_trades(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		enabled: Leverages,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &pair).unwrap_or_default();
		pool.enabled_trades = enabled;
		LiquidityPoolOptions::insert(&pool_id, &pair, pool);
		Ok(())
	}

	fn _accumulate_rates(pair: TradingPair) {
		for pool_id in <Owners<T>>::iter().map(|(_, (_, pool_id))| pool_id) {
			let long_rate = Self::get_swap_rate(pool_id, pair, true);
			let short_rate = Self::get_swap_rate(pool_id, pair, false);

			let mut accumulated = Self::accumulated_swap_rate(pool_id, pair);
			let one = Fixed128::from_natural(1);
			// acc_long_rate = 1 - ((accumulated - 1) * (-1 + rate))
			accumulated.long = one.saturating_sub(
				accumulated
					.long
					.saturating_sub(one)
					.saturating_mul(long_rate.saturating_sub(one)),
			);
			// acc_short_rate = (accumulated + 1) * (1 + rate) - 1
			accumulated.short = accumulated
				.short
				.saturating_add(one)
				.saturating_mul(one.saturating_add(short_rate))
				.saturating_sub(one);
			AccumulatedSwapRates::insert(pool_id, pair, accumulated.clone());
			Self::deposit_event(RawEvent::AccumulatedSwapRateUpdated(pool_id, pair, accumulated))
		}
	}
}

impl<T: Trait> OnDisableLiquidityPool for Module<T> {
	fn on_disable(pool_id: LiquidityPoolId) {
		LiquidityPoolOptions::remove_prefix(&pool_id);
	}
}

impl<T: Trait> OnRemoveLiquidityPool for Module<T> {
	fn on_remove(pool_id: LiquidityPoolId) {
		LiquidityPoolOptions::remove_prefix(&pool_id);
		AccumulatedSwapRates::remove_prefix(&pool_id);
		AdditionalSwapRate::remove(&pool_id);
		LiquidityPoolEnabledTradingPairs::remove_prefix(&pool_id);
		MinLeveragedAmount::remove(pool_id);
	}
}