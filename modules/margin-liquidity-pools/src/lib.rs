#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use codec::{Decode, Encode, FullCodec};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter};
use frame_system::{self as system, ensure_root, ensure_signed};
use orml_traits::{BasicCurrency, MultiCurrency};
use orml_utilities::Fixed128;
use primitives::{AccumulateConfig, Balance, CurrencyId, Leverage, Leverages, LiquidityPoolId, TradingPair};
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32Bit, CheckedAdd, EnsureOrigin, MaybeSerializeDeserialize, Member, One, Saturating,
	},
	DispatchResult, ModuleId, PerThing, Permill, RuntimeDebug,
};
use sp_std::{prelude::*, result};
use traits::{LiquidityPoolManager, LiquidityPools, MarginProtocolLiquidityPools};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct MarginLiquidityPoolOption {
	pub bid_spread: Permill,
	pub ask_spread: Permill,
	pub enabled_trades: Leverages,
}

const MODULE_ID: ModuleId = ModuleId(*b"lami/mlp");

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Balance>;
	type LiquidityPoolId: FullCodec
		+ Parameter
		+ Member
		+ Copy
		+ Ord
		+ Default
		+ AtLeast32Bit
		+ MaybeSerializeDeserialize;
	type PoolManager: LiquidityPoolManager<Self::LiquidityPoolId, Balance>;
	type ExistentialDeposit: Get<Balance>;
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
	type MaxSwap: Get<Fixed128>;
}

decl_storage! {
	trait Store for Module<T: Trait> as MarginLiquidityPools {
		pub NextPoolId get(fn next_pool_id): T::LiquidityPoolId;
		pub Owners get(fn owners): map hasher(blake2_256) T::LiquidityPoolId => Option<(T::AccountId, T::LiquidityPoolId)>;
		pub LiquidityPoolOptions get(fn liquidity_pool_options): double_map hasher(blake2_256) T::LiquidityPoolId, hasher(blake2_256) TradingPair => Option<MarginLiquidityPoolOption>;
		pub Balances get(fn balances): map hasher(blake2_256) T::LiquidityPoolId => Balance;
		pub SwapRates get(fn swap_rate): double_map hasher(blake2_256) T::LiquidityPoolId, hasher(blake2_256) TradingPair => Fixed128;
		pub AccumulatedSwapRates get(fn accumulated_swap_rate): double_map hasher(blake2_256) T::LiquidityPoolId, hasher(blake2_256) TradingPair => Fixed128;
		pub MaxSpread get(fn max_spread): map hasher(blake2_256) TradingPair => Permill;
		pub Accumulates get(fn accumulate): map hasher(blake2_256) TradingPair => Option<(AccumulateConfig<T::BlockNumber>, TradingPair)>;
		pub EnabledTradingPairs get(fn enabled_trading_pair): map hasher(blake2_256) TradingPair => Option<TradingPair>;
		pub LiquidityPoolEnabledTradingPairs get(fn liquidity_pool_enabled_trading_pair): double_map hasher(blake2_256) T::LiquidityPoolId, hasher(blake2_256) TradingPair => Option<TradingPair>;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::LiquidityPoolId,
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
		/// Swap rate updated (who, pool_id, pair, swap_rate)
		SwapRateUpdated(AccountId, LiquidityPoolId, TradingPair, Fixed128),
		/// Accumulated swap rate updated (pool_id, pair, accumulated_swap_rate)
		AccumulatedSwapRateUpdated(LiquidityPoolId, TradingPair, Fixed128),
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

		pub fn disable_pool(origin, pool_id: T::LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_disable_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolDisabled(who, pool_id));
		}

		pub fn remove_pool(origin, pool_id: T::LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_remove_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolRemoved(who, pool_id));
		}

		pub fn deposit_liquidity(origin, pool_id: T::LiquidityPoolId, amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_deposit_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::DepositLiquidity(who, pool_id, amount));
		}

		pub fn withdraw_liquidity(origin, pool_id: T::LiquidityPoolId, amount: Balance) {
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

		pub fn set_spread(origin, pool_id: T::LiquidityPoolId, pair: TradingPair, bid: Permill, ask: Permill) {
			let who = ensure_signed(origin)?;
			Self::_set_spread(&who, pool_id, pair, bid, ask)?;
			Self::deposit_event(RawEvent::SetSpread(who, pool_id, pair, bid, ask));
		}

		pub fn set_enabled_trades(origin, pool_id: T::LiquidityPoolId, pair: TradingPair, enabled: Leverages) {
			let who = ensure_signed(origin)?;
			Self::_set_enabled_trades(&who, pool_id, pair, enabled)?;
			Self::deposit_event(RawEvent::SetEnabledTrades(who, pool_id, pair, enabled));
		}

		pub fn update_swap(origin, pool_id: T::LiquidityPoolId, pair: TradingPair, rate: Fixed128) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
			ensure!(rate <= T::MaxSwap::get(), Error::<T>::SwapRateTooHigh);
			<SwapRates<T>>::insert(pool_id, pair, rate);
			Self::deposit_event(RawEvent::SwapRateUpdated(who, pool_id, pair, rate));
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

		pub fn liquidity_pool_enable_trading_pair(origin, pool_id: T::LiquidityPoolId, pair: TradingPair) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
			ensure!(Self::enabled_trading_pair(&pair).is_some(), Error::<T>::TradingPairNotEnabled);
			<LiquidityPoolEnabledTradingPairs<T>>::insert(&pool_id, &pair, &pair);
			Self::deposit_event(RawEvent::LiquidityPoolTradingPairEnabled(pair))
		}

		pub fn liquidity_pool_disable_trading_pair(origin, pool_id: T::LiquidityPoolId, pair: TradingPair) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
			<LiquidityPoolEnabledTradingPairs<T>>::remove(&pool_id, &pair);
			Self::deposit_event(RawEvent::LiquidityPoolTradingPairDisabled(pair))
		}

		fn on_initialize(n: T::BlockNumber) {
			for (accumulate_config, pair) in <Accumulates<T>>::iter() {
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
		SpreadTooHigh,
		PoolNotFound,
		TradingPairNotEnabled,
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn is_owner(pool_id: T::LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::owners(pool_id).map_or(false, |(id, _)| &id == who)
	}

	pub fn is_enabled(pool_id: T::LiquidityPoolId, pair: TradingPair, leverage: Leverage) -> bool {
		Self::liquidity_pool_options(&pool_id, &pair).map_or(false, |pool| pool.enabled_trades.contains(leverage))
	}
}

impl<T: Trait> LiquidityPools<T::AccountId> for Module<T> {
	type LiquidityPoolId = T::LiquidityPoolId;
	type CurrencyId = CurrencyId;
	type Balance = Balance;

	fn ensure_liquidity(pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult {
		ensure!(Self::balances(&pool_id) >= amount, Error::<T>::CannotWithdrawAmount);
		T::PoolManager::ensure_can_withdraw(pool_id, amount)
	}

	fn is_owner(pool_id: Self::LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::is_owner(pool_id, who)
	}

	/// Check collateral balance of `pool_id`.
	fn liquidity(pool_id: Self::LiquidityPoolId) -> Self::Balance {
		Self::balances(&pool_id)
	}

	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(
		source: &T::AccountId,
		pool_id: Self::LiquidityPoolId,
		amount: Self::Balance,
	) -> DispatchResult {
		Self::_deposit_liquidity(source, pool_id, amount)
	}

	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(
		dest: &T::AccountId,
		pool_id: Self::LiquidityPoolId,
		amount: Self::Balance,
	) -> DispatchResult {
		Self::_withdraw_liquidity(dest, pool_id, amount)
	}
}

impl<T: Trait> MarginProtocolLiquidityPools<T::AccountId> for Module<T> {
	type TradingPair = TradingPair;

	fn is_allowed_position(pool_id: Self::LiquidityPoolId, pair: TradingPair, leverage: Leverage) -> bool {
		Self::is_enabled(pool_id, pair, leverage)
	}

	fn get_bid_spread(pool_id: Self::LiquidityPoolId, pair: TradingPair) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &pair).map(|pool| pool.bid_spread)
	}

	fn get_ask_spread(pool_id: Self::LiquidityPoolId, pair: TradingPair) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &pair).map(|pool| pool.ask_spread)
	}

	fn get_swap_rate(pool_id: Self::LiquidityPoolId, pair: Self::TradingPair) -> Fixed128 {
		Self::swap_rate(pool_id, pair)
	}

	fn get_accumulated_swap_rate(pool_id: Self::LiquidityPoolId, pair: Self::TradingPair) -> Fixed128 {
		Self::accumulated_swap_rate(pool_id, pair)
	}

	fn can_open_position(
		pool_id: Self::LiquidityPoolId,
		pair: Self::TradingPair,
		leverage: Leverage,
		_leveraged_amount: Balance,
	) -> bool {
		// FIXME: this implementation may change
		Self::is_enabled(pool_id, pair, leverage)
			&& Self::enabled_trading_pair(&pair).is_some()
			&& Self::liquidity_pool_enabled_trading_pair(&pool_id, &pair).is_some()
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _create_pool(who: &T::AccountId) -> result::Result<T::LiquidityPoolId, Error<T>> {
		let pool_id = Self::next_pool_id();
		// increment next pool id
		let next_pool_id = pool_id
			.checked_add(&One::one())
			.ok_or(Error::<T>::CannotCreateMorePool)?;
		<NextPoolId<T>>::put(next_pool_id);
		// owner reference
		<Owners<T>>::insert(&pool_id, (who, pool_id));
		Ok(pool_id)
	}

	fn _disable_pool(who: &T::AccountId, pool_id: T::LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		<LiquidityPoolOptions<T>>::remove_prefix(&pool_id);
		Ok(())
	}

	fn _remove_pool(who: &T::AccountId, pool_id: T::LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		ensure!(T::PoolManager::can_remove(pool_id), Error::<T>::CannotRemovePool);

		let balance = Self::balances(&pool_id);
		// transfer balance to pool owner
		T::LiquidityCurrency::transfer(&Self::account_id(), who, balance)?;

		<Balances<T>>::remove(&pool_id);
		<Owners<T>>::remove(&pool_id);
		<LiquidityPoolOptions<T>>::remove_prefix(&pool_id);
		<SwapRates<T>>::remove_prefix(&pool_id);
		<AccumulatedSwapRates<T>>::remove_prefix(&pool_id);

		Ok(())
	}

	fn _deposit_liquidity(who: &T::AccountId, pool_id: T::LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(<Owners<T>>::contains_key(&pool_id), Error::<T>::PoolNotFound);
		let balance = Self::balances(&pool_id);
		let new_balance = balance.checked_add(amount).ok_or(Error::<T>::CannotDepositAmount)?;
		// transfer amount to this pool
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount)?;
		// update balance
		<Balances<T>>::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _withdraw_liquidity(who: &T::AccountId, pool_id: T::LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(<Owners<T>>::contains_key(&pool_id), Error::<T>::PoolNotFound);

		Self::ensure_liquidity(pool_id, amount)?;
		let new_balance = Self::balances(&pool_id)
			.checked_sub(amount)
			.ok_or(Error::<T>::CannotWithdrawAmount)?;

		// transfer amount to account
		T::LiquidityCurrency::transfer(&Self::account_id(), who, amount)?;

		// update balance
		<Balances<T>>::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _set_spread(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
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
		<LiquidityPoolOptions<T>>::insert(&pool_id, &pair, pool);
		Ok(())
	}

	fn _set_enabled_trades(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		pair: TradingPair,
		enabled: Leverages,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &pair).unwrap_or_default();
		pool.enabled_trades = enabled;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &pair, pool);
		Ok(())
	}

	fn _accumulate_rates(pair: TradingPair) {
		for pool_id in <Owners<T>>::iter().map(|(_, pool_id)| pool_id) {
			let rate = Self::swap_rate(pool_id, pair);
			let accumulated = Self::accumulated_swap_rate(pool_id, pair);
			let one = Fixed128::from_natural(1);
			// acc_rate = (accumulated + 1) * (1 + rate) - 1
			let acc_rate = accumulated
				.saturating_add(one)
				.saturating_mul(one.saturating_add(rate))
				.saturating_sub(one);
			<AccumulatedSwapRates<T>>::insert(pool_id, pair, acc_rate);
			Self::deposit_event(RawEvent::AccumulatedSwapRateUpdated(pool_id, pair, acc_rate))
		}
	}
}
