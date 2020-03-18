#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure};
use sp_arithmetic::{
	traits::{Bounded, Saturating},
	Permill,
};
use sp_runtime::{traits::StaticLookup, DispatchError, DispatchResult, RuntimeDebug};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, PriceProvider};
use orml_utilities::{Fixed128, FixedU128};
use primitives::{
	arithmetic::{fixed_128_from_fixed_u128, fixed_128_from_u128, fixed_128_mul_signum, u128_from_fixed_128},
	Balance, CurrencyId, Leverage, LiquidityPoolId, Price, TradingPair,
};
use sp_std::{cmp, prelude::*, result};
use traits::{LiquidityPoolManager, LiquidityPools, MarginProtocolLiquidityPools};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

mod mock;
mod tests;

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
	margin_call: Permill,
	stop_out: Permill,
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
		/// Position opened: (who, pool_id, trading_pair, leverage, leveraged_amount, price)
		PositionOpened(AccountId, LiquidityPoolId, TradingPair, Leverage, Amount, Price),
		/// Position closed: (who, position_id, price)
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
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		NoPrice,
		NoAskSpread,
		MarketPriceTooHigh,
		MarketPriceTooLow,
		NumOutOfBound,
		UnsafeTrader,
		TraderWouldBeUnsafe,
		UnsafePool,
		PoolWouldBeUnsafe,
		SafeTrader,
		NotReachedRiskThreshold,
		MarginCalledTrader,
		MarginCalledPool,
		NoAvailablePositionId,
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

			Self::deposit_event(RawEvent::PositionOpened(who, pool, pair, leverage, leveraged_amount, price));
		}

		pub fn close_position(origin, position_id: PositionId, price: Price) {
			let who = ensure_signed(origin)?;
			Self::_close_position(&who, position_id, Some(price))?;

			Self::deposit_event(RawEvent::PositionClosed(who, position_id, price));
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
			let who = T::Lookup::lookup(who)?;

			Self::_trader_margin_call(&who)?;
			Self::deposit_event(RawEvent::TraderMarginCalled(who));
		}

		pub fn trader_become_safe(origin, who: <T::Lookup as StaticLookup>::Source) {
			let who = T::Lookup::lookup(who)?;

			Self::_trader_become_safe(&who)?;
			Self::deposit_event(RawEvent::TraderBecameSafe(who));
		}

		pub fn trader_liquidate(origin, who: <T::Lookup as StaticLookup>::Source) {
			let who = T::Lookup::lookup(who)?;

			Self::_trader_liquidate(&who)?;
			Self::deposit_event(RawEvent::TraderLiquidated(who));
		}

		// TODO: implementations
		pub fn liquidity_pool_margin_call(origin, pool: LiquidityPoolId) {}
		pub fn liquidity_pool_become_safe(origin, pool: LiquidityPoolId) {}
		pub fn liquidity_pool_liquidate(origin, pool: LiquidityPoolId) {}
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

		Self::_ensure_trader_safe(who, Some(position.clone()))?;
		Self::_ensure_pool_safe(pool, Some(position.clone()))?;

		Self::_insert_position(who, pool, pair, position)?;

		Ok(())
	}

	fn _close_position(who: &T::AccountId, position_id: PositionId, price: Option<Price>) -> DispatchResult {
		// TODO: implementation
		unimplemented!()
	}

	fn _deposit(who: &T::AccountId, amount: Balance) -> DispatchResult {
		// TODO: implementation
		unimplemented!()
	}

	fn _withdraw(who: &T::AccountId, amount: Balance) -> DispatchResult {
		// TODO: implementation
		unimplemented!()
	}

	fn _trader_margin_call(who: &T::AccountId) -> DispatchResult {
		if !<MarginCalledTraders<T>>::contains_key(who) {
			if Self::_ensure_trader_safe(who, None).is_err() {
				<MarginCalledTraders<T>>::insert(who, ());
			} else {
				return Err(Error::<T>::SafeTrader.into());
			}
		}
		Ok(())
	}

	fn _trader_become_safe(who: &T::AccountId) -> DispatchResult {
		if <MarginCalledTraders<T>>::contains_key(who) {
			if Self::_ensure_trader_safe(who, None).is_ok() {
				<MarginCalledTraders<T>>::remove(who);
			} else {
				return Err(Error::<T>::UnsafeTrader.into());
			}
		}
		Ok(())
	}

	fn _trader_liquidate(who: &T::AccountId) -> DispatchResult {
		let threshold = TraderRiskThreshold::get();
		let margin_level = Self::_margin_level(who, None)?;

		if margin_level > threshold.stop_out.into() {
			return Err(Error::<T>::NotReachedRiskThreshold.into());
		}

		// Close position as much as possible
		// TODO: print error log
		<PositionsByTrader<T>>::iter_prefix(who).for_each(|user_position_ids| {
			user_position_ids.iter().for_each(|position_id| {
				let _ = Self::_close_position(who, *position_id, None);
			})
		});

		if Self::_ensure_trader_safe(who, None).is_ok() && <MarginCalledTraders<T>>::contains_key(who) {
			<MarginCalledTraders<T>>::remove(who);
		}
		Ok(())
	}
}

// Storage helpers
impl<T: Trait> Module<T> {
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
type BalanceResult = result::Result<Balance, DispatchError>;

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
			.ok_or(Error::<T>::NoAskSpread)?
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
	/// Unrealized profit and loss of a position(USD value).
	///
	/// unrealized_pl_of_position = (curr_price - open_price) * leveraged_held * price
	fn _unrealized_pl_of_position(position: &Position<T>) -> Fixed128Result {
		// open_price = abs(leveraged_debits / leveraged_held)
		let open_price = position
			.leveraged_debits
			.checked_div(&position.leveraged_held)
			.expect("ensured safe on open position")
			.saturating_abs();
		let curr_price = {
			if position.leverage.is_long() {
				Self::_bid_price(position.pool, position.pair, None)?
			} else {
				Self::_ask_price(position.pool, position.pair, None)?
			}
		};
		let price_delta = curr_price
			.checked_sub(&open_price)
			.expect("Non-negative integers sub can't overflow; qed");
		let unrealized = position
			.leveraged_held
			.checked_mul(&price_delta)
			.ok_or(Error::<T>::NumOutOfBound)?;
		Self::_usd_value(position.pair.base, unrealized)
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
		position
			.leveraged_held
			.saturating_abs()
			.checked_mul(&rate)
			.ok_or(Error::<T>::NumOutOfBound.into())
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
	/// If `new_position` is `None`, return the margin level based on current positions,
	/// else based on current positions plus this new one.
	fn _margin_level(who: &T::AccountId, new_position: Option<Position<T>>) -> Fixed128Result {
		let equity = Self::_equity_of_trader(who)?;
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

	/// Ensure a trader is safe, based on opened positions, or plus a new one to open.
	///
	/// Return `Ok` if ensured safe, or `Err` if not.
	fn _ensure_trader_safe(who: &T::AccountId, new_position: Option<Position<T>>) -> DispatchResult {
		let has_new = new_position.is_some();
		let margin_level = Self::_margin_level(who, new_position.clone())?;
		let not_safe = margin_level <= Self::trader_risk_threshold().margin_call.into();
		if not_safe {
			let err = if has_new {
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

	/// ENP and ELL. If `new_position` is `None`, return the ENP & ELL based on current positions,
	/// else based on current positions plus this new one.
	///
	/// ENP - Equity to Net Position ratio of a liquidity pool.
	/// ELL - Equity to Longest Leg ratio of a liquidity pool.
	fn _enp_and_ell(
		pool: LiquidityPoolId,
		new_position: Option<Position<T>>,
	) -> result::Result<(Fixed128, Fixed128), DispatchError> {
		let equity = Self::_equity_of_pool(pool)?;
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

		let enp = equity
			.checked_div(&net.saturating_abs())
			// if `net_position` is zero, ENP is max
			.unwrap_or(Fixed128::max_value());
		let longest_leg = cmp::max(positive, non_positive.saturating_abs());
		let ell = equity
			.checked_div(&longest_leg)
			// if `longest_leg` is zero, ELL is max
			.unwrap_or(Fixed128::max_value());

		Ok((enp, ell))
	}

	/// Ensure a liquidity pool is safe, based on opened positions, or plus a new one to open.
	///
	/// Return `Ok` if ensured safe, or `Err` if not.
	fn _ensure_pool_safe(pool: LiquidityPoolId, new_position: Option<Position<T>>) -> DispatchResult {
		let has_new = new_position.is_some();
		let (enp, ell) = Self::_enp_and_ell(pool, new_position)?;
		let not_safe = enp <= Self::liquidity_pool_enp_threshold().margin_call.into()
			|| ell <= Self::liquidity_pool_ell_threshold().margin_call.into();
		if not_safe {
			let err = if has_new {
				Error::<T>::PoolWouldBeUnsafe
			} else {
				Error::<T>::UnsafePool
			};
			Err(err.into())
		} else {
			Ok(())
		}
	}
}

//TODO: implementations, prevent open new position for margin called pools
impl<T: Trait> LiquidityPoolManager<LiquidityPoolId, Balance> for Module<T> {
	fn can_remove(pool: LiquidityPoolId) -> bool {
		unimplemented!()
	}

	fn get_required_deposit(pool: LiquidityPoolId) -> Balance {
		unimplemented!()
	}
}
