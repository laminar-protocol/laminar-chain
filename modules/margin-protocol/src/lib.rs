#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{debug, decl_error, decl_event, decl_module, decl_storage, ensure, IsSubType};
use sp_arithmetic::{
	traits::{Bounded, Saturating},
	Permill,
};
use sp_runtime::{
	offchain::{storage::StorageValueRef, Duration, Timestamp},
	traits::{AccountIdConversion, StaticLookup, UniqueSaturatedInto},
	transaction_validity::{InvalidTransaction, TransactionPriority, TransactionValidity, ValidTransaction},
	DispatchError, DispatchResult, ModuleId, RuntimeDebug,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system as system;
use frame_system::{ensure_none, ensure_signed, offchain::SubmitUnsignedTransaction};
use orml_traits::{MultiCurrency, PriceProvider};
use orml_utilities::{Fixed128, FixedU128};
use primitives::{
	arithmetic::{fixed_128_from_fixed_u128, fixed_128_from_u128, fixed_128_mul_signum, u128_from_fixed_128},
	Balance, CurrencyId, Leverage, LiquidityPoolId, Price, TradingPair,
};
use sp_std::{cmp, prelude::*, result};
use traits::{LiquidityPoolManager, LiquidityPools, MarginProtocolLiquidityPools, Treasury};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

mod mock;
mod tests;

const MODULE_ID: ModuleId = ModuleId(*b"lami/mgn");

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	type LiquidityPools: MarginProtocolLiquidityPools<
		Self::AccountId,
		CurrencyId = CurrencyId,
		Balance = Balance,
		LiquidityPoolId = LiquidityPoolId,
		TradingPair = TradingPair,
	>;
	type PriceProvider: PriceProvider<CurrencyId, Price>;
	type Treasury: Treasury<Self::AccountId>;
	type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
	type Call: From<Call<Self>> + IsSubType<Module<Self>, Self>;
}

pub type PositionId = u64;

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
pub struct Position<T: Trait> {
	owner: T::AccountId,
	pool: LiquidityPoolId,
	pair: TradingPair,
	leverage: Leverage,
	leveraged_held: Fixed128,
	leveraged_debits: Fixed128,
	/// USD value of leveraged debits on open position.
	leveraged_debits_in_usd: Fixed128,
	open_accumulated_swap_rate: Fixed128,
	open_margin: Balance,
}

//TODO: set this value
const MAX_POSITIONS_COUNT: u16 = u16::max_value();

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct RiskThreshold {
	pub margin_call: Permill,
	pub stop_out: Permill,
}

//TODO: Refactor `PositionsByPool` to `double_map LiquidityPoolId, (TradingPair, PositionId) => Option<()>`
// once iteration on key and values of `StorageDoubleMap` ready.
decl_storage! {
	trait Store for Module<T: Trait> as MarginProtocol {
		NextPositionId get(next_position_id): PositionId;
		Positions get(positions): map hasher(blake2_256) PositionId => Option<Position<T>>;
		PositionsByTrader get(positions_by_trader): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) LiquidityPoolId => Vec<PositionId>;
		PositionsByPool get(positions_by_pool): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) TradingPair => Vec<PositionId>;
		// SwapPeriods get(swap_periods): map hasher(black2_256) TradingPair => Option<SwapPeriod>;
		Balances get(balances): map hasher(blake2_256) T::AccountId => Balance;
		MinLiquidationPercent get(min_liquidation_percent): map hasher(blake2_256) TradingPair => Fixed128;
		MarginCalledTraders get(margin_called_traders): map hasher(blake2_256) T::AccountId => Option<()>;
		MarginCalledPools get(margin_called_pools): map hasher(blake2_256) LiquidityPoolId => Option<()>;
		TraderRiskThreshold get(trader_risk_threshold) config(): RiskThreshold;
		LiquidityPoolENPThreshold get(liquidity_pool_enp_threshold) config(): RiskThreshold;
		LiquidityPoolELLThreshold get(liquidity_pool_ell_threshold) config(): RiskThreshold;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		LiquidityPoolId = LiquidityPoolId,
		TradingPair = TradingPair,
		Amount = Balance
	{
		/// Position opened: (who, pool_id, trading_pair, leverage, leveraged_amount, market_price)
		PositionOpened(AccountId, LiquidityPoolId, TradingPair, Leverage, Amount, Price),
		/// Position closed: (who, position_id, market_price)
		PositionClosed(AccountId, PositionId, Price),
		/// Deposited: (who, amount)
		Deposited(AccountId, Amount),
		/// Withdrew: (who, amount)
		Withdrew(AccountId, Amount),
		/// TraderMarginCalled: (who)
		TraderMarginCalled(AccountId),
		/// TraderBecameSafe: (who)
		TraderBecameSafe(AccountId),
		/// TraderLiquidated: (who)
		TraderLiquidated(AccountId),
		/// LiquidityPoolMarginCalled: (pool_id)
		LiquidityPoolMarginCalled(LiquidityPoolId),
		/// LiquidityPoolBecameSafe: (pool_id)
		LiquidityPoolBecameSafe(LiquidityPoolId),
		/// LiquidityPoolLiquidated: (pool_id)
		LiquidityPoolLiquidated(LiquidityPoolId),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		NoPrice,
		NoAskSpread,
		NoBidSpread,
		MarketPriceTooHigh,
		MarketPriceTooLow,
		NumOutOfBound,
		UnsafeTrader,
		TraderWouldBeUnsafe,
		UnsafePool,
		PoolWouldBeUnsafe,
		SafeTrader,
		SafePool,
		NotReachedRiskThreshold,
		MarginCalledTrader,
		MarginCalledPool,
		NoAvailablePositionId,
		PositionNotFound,
		PositionNotOpenedByTrader,
		BalanceTooLow,
		PositionNotAllowed,
		CannotOpenPosition,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		pub fn open_position(
			origin,
			pool: LiquidityPoolId,
			pair: TradingPair,
			leverage: Leverage,
			#[compact] leveraged_amount: Balance,
			price: Price,
		) {
			let who = ensure_signed(origin)?;
			Self::_open_position(&who, pool, pair, leverage, leveraged_amount, price)?;
		}

		pub fn close_position(origin, position_id: PositionId, price: Price) {
			let who = ensure_signed(origin)?;
			Self::_close_position(&who, position_id, Some(price))?;
		}

		pub fn deposit(origin, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_deposit(&who, amount)?;

			Self::deposit_event(RawEvent::Deposited(who, amount));
		}

		pub fn withdraw(origin, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_withdraw(&who, amount)?;

			Self::deposit_event(RawEvent::Withdrew(who, amount));
		}

		pub fn trader_margin_call(origin, who: <T::Lookup as StaticLookup>::Source) {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::_trader_margin_call(&who)?;
			Self::deposit_event(RawEvent::TraderMarginCalled(who));
		}

		pub fn trader_become_safe(origin, who: <T::Lookup as StaticLookup>::Source) {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::_trader_become_safe(&who)?;
			Self::deposit_event(RawEvent::TraderBecameSafe(who));
		}

		pub fn trader_liquidate(origin, who: <T::Lookup as StaticLookup>::Source) {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::_trader_liquidate(&who)?;
			Self::deposit_event(RawEvent::TraderLiquidated(who));
		}

		pub fn liquidity_pool_margin_call(origin, pool: LiquidityPoolId) {
			ensure_none(origin)?;
			Self::_liquidity_pool_margin_call(pool)?;
			Self::deposit_event(RawEvent::LiquidityPoolMarginCalled(pool));
		}

		pub fn liquidity_pool_become_safe(origin, pool: LiquidityPoolId) {
			ensure_none(origin)?;
			Self::_liquidity_pool_become_safe(pool)?;
			Self::deposit_event(RawEvent::LiquidityPoolBecameSafe(pool));
		}

		pub fn liquidity_pool_liquidate(origin, pool: LiquidityPoolId) {
			ensure_none(origin)?;
			Self::_liquidity_pool_liquidate(pool)?;
			Self::deposit_event(RawEvent::LiquidityPoolLiquidated(pool));
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			if let Err(error) = Self::_offchain_worker(block_number) {
				match error {
					OffchainErr::NotValidator | OffchainErr::FailedToAcquireLock => {
						debug::native::info!(
							target: TAG,
							"{:?} [block_number = {:?}]",
							error,
							block_number,
						);
					},
					_ => {
						debug::native::error!(
							target: TAG,
							"{:?} [block_number = {:?}]",
							error,
							block_number,
						);
					}
				};
			}
		}
	}
}

// Dispatchable functions impl
impl<T: Trait> Module<T> {
	fn _open_position(
		who: &T::AccountId,
		pool: LiquidityPoolId,
		pair: TradingPair,
		leverage: Leverage,
		leveraged_amount: Balance,
		price: Price,
	) -> DispatchResult {
		ensure!(
			Self::margin_called_traders(who).is_none(),
			Error::<T>::MarginCalledTrader
		);
		ensure!(Self::margin_called_pools(pool).is_none(), Error::<T>::MarginCalledPool);
		ensure!(
			T::LiquidityPools::is_allowed_position(pool, pair, leverage),
			Error::<T>::PositionNotAllowed
		);

		let (held_signum, debit_signum): (i128, i128) = if leverage.is_long() { (1, -1) } else { (-1, 1) };
		let leveraged_held = fixed_128_from_u128(leveraged_amount);
		let debits_price = {
			if leverage.is_long() {
				Self::_ask_price(pool, pair, Some(price))?
			} else {
				Self::_bid_price(pool, pair, Some(price))?
			}
		};
		let leveraged_debits = leveraged_held
			.checked_mul(&debits_price)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let leveraged_held_in_usd = Self::_usd_value(pair.base, leveraged_debits)?;
		ensure!(
			T::LiquidityPools::can_open_position(pool, pair, leverage, u128_from_fixed_128(leveraged_held_in_usd)),
			Error::<T>::CannotOpenPosition
		);

		let open_margin = {
			let leverage_value = Fixed128::from_natural(leverage.value().into());
			let m = leveraged_held_in_usd
				.checked_div(&leverage_value)
				.expect("leveraged value cannot be zero; qed");
			u128_from_fixed_128(m)
		};
		let open_accumulated_swap_rate = T::LiquidityPools::get_accumulated_swap_rate(pool, pair);
		let position: Position<T> = Position {
			owner: who.clone(),
			pool,
			pair,
			leverage,
			leveraged_held: fixed_128_mul_signum(leveraged_held, held_signum),
			leveraged_debits: fixed_128_mul_signum(leveraged_debits, debit_signum),
			leveraged_debits_in_usd: fixed_128_mul_signum(leveraged_held_in_usd, debit_signum),
			open_accumulated_swap_rate,
			open_margin,
		};

		Self::_ensure_trader_safe(who, Some(position.clone()), None)?;
		Self::_ensure_pool_safe(pool, Some(position.clone()), None)?;

		Self::_insert_position(who, pool, pair, position)?;

		Self::deposit_event(RawEvent::PositionOpened(
			who.clone(),
			pool,
			pair,
			leverage,
			leveraged_amount,
			FixedU128::from_parts(u128_from_fixed_128(debits_price)),
		));

		Ok(())
	}

	fn _close_position(who: &T::AccountId, position_id: PositionId, price: Option<Price>) -> DispatchResult {
		let position = Self::positions(position_id).ok_or(Error::<T>::PositionNotFound)?;
		let index = Self::positions_by_trader(who, position.pool)
			.iter()
			.position(|id| *id == position_id)
			.ok_or(Error::<T>::PositionNotOpenedByTrader)?;
		let (unrealized_pl, market_price) = Self::_unrealized_pl_and_market_price_of_position(&position, price)?;
		let accumulated_swap_rate = Self::_accumulated_swap_rate_of_position(&position)?;
		let balance_delta = unrealized_pl
			.checked_add(&accumulated_swap_rate)
			.ok_or(Error::<T>::NumOutOfBound)?;

		// realizing
		let balance_delta_abs = u128_from_fixed_128(balance_delta.saturating_abs());
		if balance_delta.is_positive() {
			// trader has profit
			let realized = cmp::min(
				<T::LiquidityPools as LiquidityPools<T::AccountId>>::liquidity(position.pool),
				balance_delta_abs,
			);
			<T::LiquidityPools as LiquidityPools<T::AccountId>>::withdraw_liquidity(
				&Self::account_id(),
				position.pool,
				realized,
			)?;
			<Balances<T>>::mutate(who, |b| *b += realized);
		} else {
			// trader has loss
			let realized = cmp::min(Self::balances(who), balance_delta_abs);
			<T::LiquidityPools as LiquidityPools<T::AccountId>>::deposit_liquidity(
				&Self::account_id(),
				position.pool,
				realized,
			)?;
			<Balances<T>>::mutate(who, |b| *b -= realized);
		}

		// remove position
		<Positions<T>>::remove(position_id);
		<PositionsByTrader<T>>::mutate(who, position.pool, |v| v.remove(index));
		PositionsByPool::mutate(position.pool, position.pair, |v| v.retain(|id| *id != position_id));

		Self::deposit_event(RawEvent::PositionClosed(
			who.clone(),
			position_id,
			FixedU128::from_parts(u128_from_fixed_128(market_price)),
		));

		Ok(())
	}

	fn _deposit(who: &T::AccountId, amount: Balance) -> DispatchResult {
		T::MultiCurrency::transfer(CurrencyId::AUSD, who, &Self::account_id(), amount)?;
		<Balances<T>>::mutate(who, |b| *b += amount);
		Ok(())
	}

	fn _withdraw(who: &T::AccountId, amount: Balance) -> DispatchResult {
		ensure!(Self::balances(who) >= amount, Error::<T>::BalanceTooLow);

		let equity_delta = fixed_128_mul_signum(fixed_128_from_u128(amount), -1);
		Self::_ensure_trader_safe(who, None, Some(equity_delta))?;

		T::MultiCurrency::transfer(CurrencyId::AUSD, &Self::account_id(), who, amount)?;
		<Balances<T>>::mutate(who, |b| *b -= amount);

		Ok(())
	}

	fn _trader_margin_call(who: &T::AccountId) -> DispatchResult {
		if !<MarginCalledTraders<T>>::contains_key(who) {
			if Self::_ensure_trader_safe(who, None, None).is_err() {
				<MarginCalledTraders<T>>::insert(who, ());
			} else {
				return Err(Error::<T>::SafeTrader.into());
			}
		}
		Ok(())
	}

	fn _trader_become_safe(who: &T::AccountId) -> DispatchResult {
		if <MarginCalledTraders<T>>::contains_key(who) {
			if Self::_ensure_trader_safe(who, None, None).is_ok() {
				<MarginCalledTraders<T>>::remove(who);
			} else {
				return Err(Error::<T>::UnsafeTrader.into());
			}
		}
		Ok(())
	}

	fn _trader_liquidate(who: &T::AccountId) -> DispatchResult {
		let threshold = TraderRiskThreshold::get();
		let margin_level = Self::_margin_level(who, None, None)?;

		if margin_level > threshold.stop_out.into() {
			return Err(Error::<T>::NotReachedRiskThreshold.into());
		}

		// Close position as much as possible
		<PositionsByTrader<T>>::iter_prefix(who).for_each(|trading_pair_position_ids| {
			trading_pair_position_ids.iter().for_each(|position_id| {
				let _ = Self::_close_position(who, *position_id, None);
			})
		});

		if Self::_ensure_trader_safe(who, None, None).is_ok() && <MarginCalledTraders<T>>::contains_key(who) {
			<MarginCalledTraders<T>>::remove(who);
		}
		Ok(())
	}

	fn _liquidity_pool_margin_call(pool: LiquidityPoolId) -> DispatchResult {
		if !MarginCalledPools::contains_key(pool) {
			if Self::_ensure_pool_safe(pool, None, None).is_err() {
				MarginCalledPools::insert(pool, ());
			} else {
				return Err(Error::<T>::SafePool.into());
			}
		}
		Ok(())
	}

	fn _liquidity_pool_become_safe(pool: LiquidityPoolId) -> DispatchResult {
		if MarginCalledPools::contains_key(pool) {
			if Self::_ensure_pool_safe(pool, None, None).is_ok() {
				MarginCalledPools::remove(pool);
			} else {
				return Err(Error::<T>::UnsafePool.into());
			}
		}
		Ok(())
	}

	fn _liquidity_pool_liquidate(pool: LiquidityPoolId) -> DispatchResult {
		let (enp, ell) = Self::_enp_and_ell(pool, None, None)?;
		let need_liquidating = enp <= Self::liquidity_pool_enp_threshold().stop_out.into()
			|| ell <= Self::liquidity_pool_ell_threshold().stop_out.into();
		if !need_liquidating {
			return Err(Error::<T>::NotReachedRiskThreshold.into());
		}

		PositionsByPool::iter_prefix(pool).for_each(|trading_pair_position_ids| {
			trading_pair_position_ids.iter().for_each(|position_id| {
				let _ = Self::_liquidity_pool_close_position(pool, *position_id);
			});
		});

		if Self::_ensure_pool_safe(pool, None, None).is_ok() && MarginCalledPools::contains_key(pool) {
			MarginCalledPools::remove(pool);
		}
		Ok(())
	}
}

// Storage helpers
impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	fn _insert_position(
		who: &T::AccountId,
		pool: LiquidityPoolId,
		pair: TradingPair,
		position: Position<T>,
	) -> DispatchResult {
		let id = Self::next_position_id();
		ensure!(id != PositionId::max_value(), Error::<T>::NoAvailablePositionId);
		NextPositionId::mutate(|id| *id += 1);

		<Positions<T>>::insert(id, position);
		<PositionsByTrader<T>>::mutate(who, pool, |ids| ids.push(id));
		PositionsByPool::mutate(pool, pair, |ids| ids.push(id));

		Ok(())
	}
}

type PriceResult = result::Result<Price, DispatchError>;
type Fixed128Result = result::Result<Fixed128, DispatchError>;

// Price helpers
impl<T: Trait> Module<T> {
	/// The price from oracle.
	fn _price(base: CurrencyId, quote: CurrencyId) -> PriceResult {
		T::PriceProvider::get_price(base, quote).ok_or(Error::<T>::NoPrice.into())
	}

	/// ask_price = price * (1 + ask_spread)
	fn _ask_price(pool: LiquidityPoolId, pair: TradingPair, max: Option<Price>) -> Fixed128Result {
		let price = Self::_price(pair.base, pair.quote)?;
		let spread: Price = T::LiquidityPools::get_ask_spread(pool, pair)
			.ok_or(Error::<T>::NoAskSpread)?
			.into();
		let ask_price: Price = Price::from_natural(1).saturating_add(spread).saturating_mul(price);

		if let Some(m) = max {
			if ask_price > m {
				return Err(Error::<T>::MarketPriceTooHigh.into());
			}
		}

		Ok(fixed_128_from_fixed_u128(ask_price))
	}

	/// bid_price = price * (1 - bid_spread)
	fn _bid_price(pool: LiquidityPoolId, pair: TradingPair, min: Option<Price>) -> Fixed128Result {
		let price = Self::_price(pair.base, pair.quote)?;
		let spread: Price = T::LiquidityPools::get_bid_spread(pool, pair)
			.ok_or(Error::<T>::NoBidSpread)?
			.into();
		let bid_price = Price::from_natural(1).saturating_sub(spread).saturating_mul(price);
		if let Some(m) = min {
			if bid_price < m {
				return Err(Error::<T>::MarketPriceTooLow.into());
			}
		}

		Ok(fixed_128_from_fixed_u128(bid_price))
	}

	/// usd_value = amount * price
	fn _usd_value(currency_id: CurrencyId, amount: Fixed128) -> Fixed128Result {
		let price = {
			let p = Self::_price(CurrencyId::AUSD, currency_id)?;
			fixed_128_from_fixed_u128(p)
		};
		amount.checked_mul(&price).ok_or(Error::<T>::NumOutOfBound.into())
	}
}

// Trader helpers
impl<T: Trait> Module<T> {
	/// Unrealized profit and loss of a position(USD value), based on current market price.
	///
	/// unrealized_pl_of_position = (curr_price - open_price) * leveraged_held * to_usd_price
	fn _unrealized_pl_of_position(position: &Position<T>) -> Fixed128Result {
		let (unrealized, _) = Self::_unrealized_pl_and_market_price_of_position(position, None)?;
		Ok(unrealized)
	}

	/// Returns `Ok((unrealized_pl, market_price))` of a given position. If `price`, market price must fit this bound,
	/// else returns `None`.
	fn _unrealized_pl_and_market_price_of_position(
		position: &Position<T>,
		price: Option<Price>,
	) -> result::Result<(Fixed128, Fixed128), DispatchError> {
		// open_price = abs(leveraged_debits / leveraged_held)
		let open_price = position
			.leveraged_debits
			.checked_div(&position.leveraged_held)
			.expect("ensured safe on open position")
			.saturating_abs();
		let curr_price = {
			if position.leverage.is_long() {
				Self::_bid_price(position.pool, position.pair, price)?
			} else {
				Self::_ask_price(position.pool, position.pair, price)?
			}
		};
		let price_delta = curr_price
			.checked_sub(&open_price)
			.expect("Non-negative integers sub can't overflow; qed");
		let unrealized = position
			.leveraged_held
			.checked_mul(&price_delta)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let usd_value = Self::_usd_value(position.pair.base, unrealized)?;

		Ok((usd_value, curr_price))
	}

	/// Unrealized profit and loss of a given trader(USD value). It is the sum of unrealized profit and loss of all positions
	/// opened by a trader.
	fn _unrealized_pl_of_trader(who: &T::AccountId) -> Fixed128Result {
		<PositionsByTrader<T>>::iter_prefix(who)
			.flatten()
			.filter_map(|position_id| Self::positions(position_id))
			.try_fold(Fixed128::zero(), |acc, p| {
				let unrealized = Self::_unrealized_pl_of_position(&p)?;
				acc.checked_add(&unrealized).ok_or(Error::<T>::NumOutOfBound.into())
			})
	}

	/// Sum of all open margin of a given trader.
	fn _margin_held(who: &T::AccountId) -> Balance {
		<PositionsByTrader<T>>::iter_prefix(who)
			.flatten()
			.filter_map(|position_id| Self::positions(position_id))
			.map(|p| p.open_margin)
			.sum()
	}

	/// Free balance: the balance available for withdraw.
	///
	/// free_balance = max(balance - margin_held, zero)
	fn _free_balance(who: &T::AccountId) -> Balance {
		Self::balances(who)
			.checked_sub(Self::_margin_held(who))
			.unwrap_or_default()
	}

	/// Accumulated swap rate of a position(USD value).
	///
	/// accumulated_swap_rate_of_position = (current_accumulated - open_accumulated) * leveraged_held
	fn _accumulated_swap_rate_of_position(position: &Position<T>) -> Fixed128Result {
		let rate = T::LiquidityPools::get_accumulated_swap_rate(position.pool, position.pair)
			.checked_sub(&position.open_accumulated_swap_rate)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let accumulated_swap = position
			.leveraged_held
			.saturating_abs()
			.checked_mul(&rate)
			.ok_or(Error::<T>::NumOutOfBound)?;

		if position.leverage.is_long() {
			Fixed128::zero()
				.checked_sub(&accumulated_swap)
				.ok_or(Error::<T>::NumOutOfBound.into())
		} else {
			Ok(accumulated_swap)
		}
	}

	/// Accumulated swap of all open positions of a given trader(USD value).
	fn _accumulated_swap_rate_of_trader(who: &T::AccountId) -> Fixed128Result {
		<PositionsByTrader<T>>::iter_prefix(who)
			.flatten()
			.filter_map(|position_id| Self::positions(position_id))
			.try_fold(Fixed128::zero(), |acc, p| {
				let rate_of_p = Self::_accumulated_swap_rate_of_position(&p)?;
				acc.checked_add(&rate_of_p).ok_or(Error::<T>::NumOutOfBound.into())
			})
	}

	/// equity_of_trader = balance + unrealized_pl + accumulated_swap_rate
	fn _equity_of_trader(who: &T::AccountId) -> Fixed128Result {
		let unrealized = Self::_unrealized_pl_of_trader(who)?;
		let with_unrealized = fixed_128_from_u128(Self::balances(who))
			.checked_add(&unrealized)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let accumulated_swap_rate = Self::_accumulated_swap_rate_of_trader(who)?;
		with_unrealized
			.checked_add(&accumulated_swap_rate)
			.ok_or(Error::<T>::NumOutOfBound.into())
	}

	/// Margin level of a given user.
	///
	/// If `new_position` is `None`, return the margin level based on current positions, else based on current
	/// positions plus this new one. If `equity_delta`, apply the delta to current equity.
	fn _margin_level(
		who: &T::AccountId,
		new_position: Option<Position<T>>,
		equity_delta: Option<Fixed128>,
	) -> Fixed128Result {
		let mut equity = Self::_equity_of_trader(who)?;
		if let Some(d) = equity_delta {
			equity = equity.checked_add(&d).ok_or(Error::<T>::NumOutOfBound)?;
		}
		let leveraged_debits_in_usd = <PositionsByTrader<T>>::iter_prefix(who)
			.flatten()
			.filter_map(|position_id| Self::positions(position_id))
			.chain(new_position.map_or(vec![], |p| vec![p]))
			.try_fold(Fixed128::zero(), |acc, p| {
				acc.checked_add(&p.leveraged_debits_in_usd.saturating_abs())
					.ok_or(Error::<T>::NumOutOfBound)
			})?;
		Ok(equity
			.checked_div(&leveraged_debits_in_usd)
			// if no leveraged held, margin level is max
			.unwrap_or(Fixed128::max_value()))
	}

	/// Ensure a trader is safe, based on equity delta, opened positions or plus a new one to open.
	///
	/// Return `Ok` if ensured safe, or `Err` if not.
	fn _ensure_trader_safe(
		who: &T::AccountId,
		new_position: Option<Position<T>>,
		equity_delta: Option<Fixed128>,
	) -> DispatchResult {
		let has_change = new_position.is_some() || equity_delta.is_some();
		let margin_level = Self::_margin_level(who, new_position.clone(), equity_delta)?;
		let not_safe = margin_level <= Self::trader_risk_threshold().margin_call.into();
		if not_safe {
			let err = if has_change {
				Error::<T>::TraderWouldBeUnsafe
			} else {
				Error::<T>::UnsafeTrader
			};
			Err(err.into())
		} else {
			Ok(())
		}
	}
}

// Liquidity pool helpers
impl<T: Trait> Module<T> {
	/// equity_of_pool = liquidity - all_unrealized_pl - all_accumulated_swap_rate
	fn _equity_of_pool(pool: LiquidityPoolId) -> Fixed128Result {
		let liquidity = {
			let l = <T::LiquidityPools as LiquidityPools<T::AccountId>>::liquidity(pool);
			fixed_128_from_u128(l)
		};

		// all_unrealized_pl + all_accumulated_swap_rate
		let unrealized_pl_and_rate = PositionsByPool::iter_prefix(pool)
			.flatten()
			.filter_map(|position_id| Self::positions(position_id))
			.try_fold::<_, _, Fixed128Result>(Fixed128::zero(), |acc, p| {
				let rate = Self::_accumulated_swap_rate_of_position(&p)?;
				let unrealized = Self::_unrealized_pl_of_position(&p)?;
				let sum = rate.checked_add(&unrealized).ok_or(Error::<T>::NumOutOfBound)?;
				acc.checked_add(&sum).ok_or(Error::<T>::NumOutOfBound.into())
			})?;

		liquidity
			.checked_sub(&unrealized_pl_and_rate)
			.ok_or(Error::<T>::NumOutOfBound.into())
	}

	/// Returns `(net_position, longest_leg)` of a liquidity pool.
	fn _net_position_and_longest_leg(pool: LiquidityPoolId, new_position: Option<Position<T>>) -> (Fixed128, Fixed128) {
		let (net, positive, non_positive) = PositionsByPool::iter_prefix(pool)
			.flatten()
			.filter_map(|position_id| Self::positions(position_id))
			.chain(new_position.map_or(vec![], |p| vec![p]))
			.fold(
				(Fixed128::zero(), Fixed128::zero(), Fixed128::zero()),
				|(net, positive, non_positive), p| {
					let new_net = net
						.checked_add(&p.leveraged_debits_in_usd)
						.expect("ensured safe on open position; qed");
					if p.leveraged_debits_in_usd.is_positive() {
						(
							new_net,
							positive
								.checked_add(&p.leveraged_debits_in_usd)
								.expect("ensured safe on open position; qed"),
							non_positive,
						)
					} else {
						(
							new_net,
							positive,
							non_positive
								.checked_add(&p.leveraged_debits_in_usd)
								.expect("ensured safe on open position; qed"),
						)
					}
				},
			);

		let net = net.saturating_abs();
		let longest_leg = cmp::max(positive, non_positive.saturating_abs());

		(net, longest_leg)
	}

	/// ENP and ELL. If `new_position` is `None`, return the ENP & ELL based on current positions,
	/// else based on current positions plus this new one. If `equity_delta` is `None`, return
	/// the ENP & ELL based on current equity of pool, else based on current equity of pool plus
	/// the `equity_delta`.
	///
	/// ENP - Equity to Net Position ratio of a liquidity pool.
	/// ELL - Equity to Longest Leg ratio of a liquidity pool.
	fn _enp_and_ell(
		pool: LiquidityPoolId,
		new_position: Option<Position<T>>,
		equity_delta: Option<Fixed128>,
	) -> result::Result<(Fixed128, Fixed128), DispatchError> {
		let mut equity = Self::_equity_of_pool(pool)?;
		if let Some(e) = equity_delta {
			equity = equity.checked_add(&e).ok_or(Error::<T>::NumOutOfBound)?;
		}

		let (net_position, longest_leg) = Self::_net_position_and_longest_leg(pool, new_position);
		let enp = equity
			.checked_div(&net_position)
			// if `net_position` is zero, ENP is max
			.unwrap_or(Fixed128::max_value());
		let ell = equity
			.checked_div(&longest_leg)
			// if `longest_leg` is zero, ELL is max
			.unwrap_or(Fixed128::max_value());

		Ok((enp, ell))
	}

	/// Ensure a liquidity pool is safe, based on opened positions, or plus a new one to open.
	///
	/// Return `Ok` if ensured safe, or `Err` if not.
	fn _ensure_pool_safe(
		pool: LiquidityPoolId,
		new_position: Option<Position<T>>,
		equity_delta: Option<Fixed128>,
	) -> DispatchResult {
		let has_change = new_position.is_some() || equity_delta.is_some();
		let (enp, ell) = Self::_enp_and_ell(pool, new_position, equity_delta)?;
		let not_safe = enp <= Self::liquidity_pool_enp_threshold().margin_call.into()
			|| ell <= Self::liquidity_pool_ell_threshold().margin_call.into();
		if not_safe {
			let err = if has_change {
				Error::<T>::PoolWouldBeUnsafe
			} else {
				Error::<T>::UnsafePool
			};
			Err(err.into())
		} else {
			Ok(())
		}
	}

	/// Force closure position to liquidate liquidity pool based on opened positions.
	///
	/// Return `Ok` if closure success, or `Err` if not.
	fn _liquidity_pool_close_position(pool: LiquidityPoolId, position_id: PositionId) -> DispatchResult {
		let position = Self::positions(position_id).ok_or(Error::<T>::PositionNotFound)?;

		let price = Self::_price(position.pair.base, position.pair.quote)?;
		let spread = {
			if position.leverage.is_long() {
				T::LiquidityPools::get_bid_spread(pool, position.pair)
					.ok_or(Error::<T>::NoBidSpread)?
					.into()
			} else {
				T::LiquidityPools::get_ask_spread(pool, position.pair)
					.ok_or(Error::<T>::NoAskSpread)?
					.into()
			}
		};

		let spread_profit = position
			.leveraged_held
			.saturating_abs()
			.checked_mul(&fixed_128_from_fixed_u128(price).saturating_mul(spread))
			.ok_or(Error::<T>::NumOutOfBound)?;

		let spread_profit_in_usd = Self::_usd_value(position.pair.base, spread_profit)?;
		let penalty = spread_profit_in_usd;
		let sub_amount = spread_profit_in_usd
			.checked_add(&penalty)
			.ok_or(Error::<T>::NumOutOfBound)?;

		Self::_close_position(&position.owner, position_id, None)?;

		let realized = cmp::min(
			<T::LiquidityPools as LiquidityPools<T::AccountId>>::liquidity(position.pool),
			u128_from_fixed_128(sub_amount),
		);
		<T::LiquidityPools as LiquidityPools<T::AccountId>>::withdraw_liquidity(
			&T::Treasury::account_id(),
			position.pool,
			realized,
		)?;

		Ok(())
	}
}

impl<T: Trait> LiquidityPoolManager<LiquidityPoolId, Balance> for Module<T> {
	/// Returns if `pool` has liability in margin protocol.
	fn can_remove(pool: LiquidityPoolId) -> bool {
		PositionsByPool::iter_prefix(pool).flatten().count() == 0
	}

	/// Returns required deposit amount to make pool safe.
	fn get_required_deposit(pool: LiquidityPoolId) -> result::Result<Balance, DispatchError> {
		let (net_position, longest_leg) = Self::_net_position_and_longest_leg(pool, None);
		let required_equity = {
			let for_enp = net_position
				.checked_mul(&Self::liquidity_pool_enp_threshold().margin_call.into())
				.expect("ENP margin call threshold < 1; qed");
			let for_ell = longest_leg
				.checked_mul(&Self::liquidity_pool_ell_threshold().margin_call.into())
				.expect("ELL margin call threshold < 1; qed");
			cmp::max(for_enp, for_ell)
		};
		let equity = Self::_equity_of_pool(pool)?;
		let gap = required_equity.checked_sub(&equity).ok_or(Error::<T>::NumOutOfBound)?;

		// would be saturated into zero if gap < 0
		return Ok(u128_from_fixed_128(gap));
	}

	fn ensure_can_withdraw(pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		let equity_delta = Fixed128::zero()
			.checked_sub(&fixed_128_from_u128(amount))
			.expect("negation; qed");
		Self::_ensure_pool_safe(pool_id, None, Some(equity_delta))
	}
}

/// Error which may occur while executing the off-chain code.
#[cfg_attr(test, derive(PartialEq))]
enum OffchainErr {
	FailedToAcquireLock,
	SubmitTransaction,
	NotValidator,
	LockStillInLocked,
	CheckFail,
}

// constant for offchain worker
const LOCK_EXPIRE_DURATION: u64 = 60_000; // 60 sec
const LOCK_UPDATE_DURATION: u64 = 40_000; // 40 sec
const DB_PREFIX: &[u8] = b"laminar/margin-protocol-offchain-worker/";
const TAG: &str = "MARGIN_PROTOCOL_OFFCHAIN_WORKER";

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::FailedToAcquireLock => write!(fmt, "Failed to acquire lock"),
			OffchainErr::SubmitTransaction => write!(fmt, "Failed to submit transaction"),
			OffchainErr::NotValidator => write!(fmt, "Not validator"),
			OffchainErr::LockStillInLocked => write!(fmt, "Lock is still in locked"),
			OffchainErr::CheckFail => write!(fmt, "Check fail"),
		}
	}
}

impl<T: Trait> Module<T> {
	/// Get a list of traders
	fn _get_traders() -> Vec<T::AccountId> {
		// TODO: use key iter after this gets closed https://github.com/paritytech/substrate/issues/5319
		let mut traders: Vec<T::AccountId> = <Positions<T>>::iter().map(|x| x.owner).collect();
		traders.sort();
		traders.dedup(); // dedup works as unique for sorted vec, so we sort first
		traders
	}

	/// Get a list of pools
	fn _get_pools() -> Vec<LiquidityPoolId> {
		// TODO: use key iter after this gets closed https://github.com/paritytech/substrate/issues/5319
		let mut pools: Vec<LiquidityPoolId> = <Positions<T>>::iter().map(|x| x.pool).collect();
		pools.sort();
		pools.dedup(); // dedup works as unique for sorted vec, so we sort first
		pools
	}

	fn _offchain_worker(block_number: T::BlockNumber) -> Result<(), OffchainErr> {
		// check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		// Acquire offchain worker lock.
		// If succeeded, update the lock, otherwise return error
		let _ = Self::_acquire_offchain_worker_lock()?;

		debug::native::trace!(target: TAG, "Started [block_number = {:?}]", block_number);

		let (stop_out, margin_call, safe) = Self::_check_all_traders()?;

		for trader in stop_out {
			let who = T::Lookup::unlookup(trader.clone());
			let call = Call::<T>::trader_liquidate(who);
			T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
			debug::native::trace!(
				target: TAG,
				"Trader liquidate [trader = {:?}, block_number = {:?}]",
				trader,
				block_number
			);
		}

		for trader in margin_call {
			if !Self::_is_trader_margin_called(&trader) {
				let who = T::Lookup::unlookup(trader.clone());
				let call = Call::<T>::trader_margin_call(who);
				T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
				debug::native::trace!(
					target: TAG,
					"Trader margin call [trader = {:?}, block_number = {:?}]",
					trader,
					block_number
				);
			}
		}

		for trader in safe {
			if Self::_is_trader_margin_called(&trader) {
				let who = T::Lookup::unlookup(trader.clone());
				let call = Call::<T>::trader_become_safe(who);
				T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
				debug::native::trace!(
					target: TAG,
					"Trader become safe [trader = {:?}, block_number = {:?}]",
					trader,
					block_number
				);
			}
		}

		Self::_extend_offchain_worker_lock_if_needed();

		let (stop_out, margin_call, safe) = Self::_check_all_pools()?;

		for pool_id in stop_out {
			let call = Call::<T>::liquidity_pool_liquidate(pool_id);
			T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
			debug::native::trace!(
				target: TAG,
				"Liquidity pool liquidate [pool_id = {:?}, block_number = {:?}]",
				pool_id,
				block_number
			);
		}

		for pool_id in margin_call {
			if !Self::_is_pool_margin_called(&pool_id) {
				let call = Call::<T>::liquidity_pool_margin_call(pool_id);
				T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
				debug::native::trace!(
					target: TAG,
					"Liquidity pool margin call [pool_id = {:?}, block_number = {:?}]",
					pool_id,
					block_number
				);
			}
		}

		for pool_id in safe {
			if Self::_is_pool_margin_called(&pool_id) {
				let call = Call::<T>::liquidity_pool_become_safe(pool_id);
				T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
				debug::native::trace!(
					target: TAG,
					"Liquidity pool become safe [pool_id = {:?}, block_number = {:?}]",
					pool_id,
					block_number
				);
			}
		}

		Self::_release_offchain_worker_lock();

		debug::native::trace!(target: TAG, "Finished [block_number = {:?}]", block_number);
		Ok(())
	}

	fn _is_trader_margin_called(who: &T::AccountId) -> bool {
		<MarginCalledTraders<T>>::contains_key(&who)
	}

	fn _is_pool_margin_called(pool_id: &LiquidityPoolId) -> bool {
		MarginCalledPools::contains_key(pool_id)
	}

	fn _should_liquidate_trader(who: &T::AccountId) -> Result<bool, OffchainErr> {
		let margin_level = Self::_margin_level(who, None, None).map_err(|_| OffchainErr::CheckFail)?;

		Ok(margin_level <= Self::trader_risk_threshold().stop_out.into())
	}

	fn _should_liquidate_liquidity_pool(pool_id: &LiquidityPoolId) -> Result<bool, OffchainErr> {
		let enp_threshold = Self::liquidity_pool_enp_threshold();
		let ell_threshold = Self::liquidity_pool_ell_threshold();

		let (enp, ell) = Self::_enp_and_ell(*pool_id, None, None).map_err(|_| OffchainErr::CheckFail)?;
		Ok(enp <= enp_threshold.stop_out.into() || ell <= ell_threshold.stop_out.into())
	}

	fn _check_all_traders() -> Result<(Vec<T::AccountId>, Vec<T::AccountId>, Vec<T::AccountId>), OffchainErr> {
		let mut stop_out: Vec<T::AccountId> = vec![];
		let mut margin_call: Vec<T::AccountId> = vec![];
		let mut safe: Vec<T::AccountId> = vec![];

		let threshold = Self::trader_risk_threshold();

		for trader in Self::_get_traders() {
			let margin_level = Self::_margin_level(&trader, None, None).map_err(|_| OffchainErr::CheckFail)?;
			if margin_level <= threshold.stop_out.into() {
				stop_out.push(trader);
			} else if margin_level <= threshold.margin_call.into() {
				margin_call.push(trader);
			} else {
				safe.push(trader);
			}
		}

		Ok((stop_out, margin_call, safe))
	}

	fn _check_all_pools() -> Result<(Vec<LiquidityPoolId>, Vec<LiquidityPoolId>, Vec<LiquidityPoolId>), OffchainErr> {
		let mut stop_out: Vec<LiquidityPoolId> = vec![];
		let mut margin_call: Vec<LiquidityPoolId> = vec![];
		let mut safe: Vec<LiquidityPoolId> = vec![];

		let enp_threshold = Self::liquidity_pool_enp_threshold();
		let ell_threshold = Self::liquidity_pool_ell_threshold();

		for pool_id in Self::_get_pools() {
			let (enp, ell) = Self::_enp_and_ell(pool_id, None, None).map_err(|_| OffchainErr::CheckFail)?;
			if enp <= enp_threshold.stop_out.into() || ell <= ell_threshold.stop_out.into() {
				stop_out.push(pool_id);
			} else if enp <= enp_threshold.margin_call.into() || ell <= ell_threshold.margin_call.into() {
				margin_call.push(pool_id);
			} else {
				safe.push(pool_id);
			}
		}

		Ok((stop_out, margin_call, safe))
	}

	fn _acquire_offchain_worker_lock() -> Result<Timestamp, OffchainErr> {
		let storage_key = DB_PREFIX.to_vec();
		let storage = StorageValueRef::persistent(&storage_key);

		let acquire_lock = storage.mutate(|lock: Option<Option<Timestamp>>| {
			let now = sp_io::offchain::timestamp();
			match lock {
				None => {
					let expire_timestamp = now.add(Duration::from_millis(LOCK_EXPIRE_DURATION));
					Ok(expire_timestamp)
				}
				Some(Some(expire_timestamp)) if now >= expire_timestamp => {
					let expire_timestamp = now.add(Duration::from_millis(LOCK_EXPIRE_DURATION));
					Ok(expire_timestamp)
				}
				_ => Err(OffchainErr::LockStillInLocked),
			}
		})?;

		acquire_lock.map_err(|_| OffchainErr::FailedToAcquireLock)
	}

	fn _release_offchain_worker_lock() {
		let storage_key = DB_PREFIX.to_vec();
		let storage = StorageValueRef::persistent(&storage_key);
		let now = sp_io::offchain::timestamp();
		storage.set(&now);
	}

	fn _extend_offchain_worker_lock_if_needed() {
		let storage_key = DB_PREFIX.to_vec();
		let storage = StorageValueRef::persistent(&storage_key);

		if let Some(Some(current_expire)) = storage.get::<Timestamp>() {
			if current_expire <= sp_io::offchain::timestamp().add(Duration::from_millis(LOCK_UPDATE_DURATION)) {
				let future_expire = sp_io::offchain::timestamp().add(Duration::from_millis(LOCK_EXPIRE_DURATION));
				storage.set(&future_expire);
			}
		}
	}
}

#[allow(deprecated)]
impl<T: Trait> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(call: &Self::Call) -> TransactionValidity {
		let base_priority = TransactionPriority::max_value() - 86400 * 365 * 100 / 2;
		let block_number = <system::Module<T>>::block_number().unique_saturated_into() as u64;

		let defaults = ValidTransaction {
			priority: base_priority.saturating_add(block_number),
			requires: vec![],
			provides: vec![],
			longevity: 64_u64,
			propagate: true,
		};

		match call {
			Call::trader_margin_call(who) => {
				let trader = T::Lookup::lookup(who.clone()).expect(InvalidTransaction::Stale.into());
				if Self::_is_trader_margin_called(&trader) {
					return InvalidTransaction::Stale.into();
				}

				Ok(ValidTransaction {
					provides: vec![("margin_protocol/trader_margin_call", who).encode()],
					..defaults
				})
			}
			Call::trader_become_safe(who) => {
				let trader = T::Lookup::lookup(who.clone()).expect(InvalidTransaction::Stale.into());
				if !Self::_is_trader_margin_called(&trader) {
					return InvalidTransaction::Stale.into();
				}

				Ok(ValidTransaction {
					provides: vec![("margin_protocol/trader_become_safe", who).encode()],
					..defaults
				})
			}
			Call::trader_liquidate(who) => {
				let trader = T::Lookup::lookup(who.clone()).expect(InvalidTransaction::Stale.into());
				if Self::_should_liquidate_trader(&trader).ok() == Some(true) {
					return Ok(ValidTransaction {
						provides: vec![("margin_protocol/trader_liquidate", who).encode()],
						..defaults
					});
				}
				InvalidTransaction::Stale.into()
			}
			Call::liquidity_pool_margin_call(pool_id) => {
				if Self::_is_pool_margin_called(pool_id) {
					return InvalidTransaction::Stale.into();
				}

				Ok(ValidTransaction {
					provides: vec![("margin_protocol/liquidity_pool_margin_call", pool_id).encode()],
					..defaults
				})
			}
			Call::liquidity_pool_become_safe(pool_id) => {
				if !Self::_is_pool_margin_called(pool_id) {
					return InvalidTransaction::Stale.into();
				}

				Ok(ValidTransaction {
					provides: vec![("margin_protocol/liquidity_pool_become_safe", pool_id).encode()],
					..defaults
				})
			}
			Call::liquidity_pool_liquidate(pool_id) => {
				if Self::_should_liquidate_liquidity_pool(pool_id).ok() == Some(true) {
					return Ok(ValidTransaction {
						provides: vec![("margin_protocol/liquidity_pool_liquidate", pool_id).encode()],
						..defaults
					});
				}

				InvalidTransaction::Stale.into()
			}
			_ => InvalidTransaction::Call.into(),
		}
	}
}
