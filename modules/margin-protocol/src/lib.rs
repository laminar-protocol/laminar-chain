#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, weights::SimpleDispatchInfo,
	IsSubType, IterableStorageDoubleMap, IterableStorageMap,
};
use sp_arithmetic::{
	traits::{Bounded, Saturating},
	Permill,
};
use sp_runtime::{
	offchain::{storage::StorageValueRef, Duration, Timestamp},
	traits::{AccountIdConversion, StaticLookup},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity, ValidTransaction,
	},
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
	type LiquidityPools: MarginProtocolLiquidityPools<Self::AccountId>;
	type PriceProvider: PriceProvider<CurrencyId, Price>;
	type Treasury: Treasury<Self::AccountId>;
	type SubmitTransaction: SubmitUnsignedTransaction<Self, <Self as Trait>::Call>;
	type Call: From<Call<Self>> + IsSubType<Module<Self>, Self>;
	type GetTraderMaxOpenPositions: Get<usize>;
	type GetPoolMaxOpenPositions: Get<usize>;
	type UnsignedPriority: Get<TransactionPriority>;
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
	margin_held: Fixed128,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct RiskThreshold {
	pub margin_call: Permill,
	pub stop_out: Permill,
}

decl_storage! {
	trait Store for Module<T: Trait> as MarginProtocol {
		NextPositionId get(next_position_id): PositionId;
		Positions get(positions): map hasher(twox_64_concat) PositionId => Option<Position<T>>;
		PositionsByTrader get(positions_by_trader): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) (LiquidityPoolId, PositionId) => Option<bool>;
		PositionsByPool get(positions_by_pool): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) (TradingPair, PositionId) => Option<bool>;
		Balances get(balances): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) LiquidityPoolId => Fixed128;
		MarginCalledTraders get(margin_called_traders): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) LiquidityPoolId => Option<bool>;
		MarginCalledPools get(margin_called_pools): map hasher(twox_64_concat) LiquidityPoolId => Option<bool>;
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
		/// Position opened: (who, position_id, pool_id, trading_pair, leverage, leveraged_amount, market_price)
		PositionOpened(AccountId, PositionId, LiquidityPoolId, TradingPair, Leverage, Amount, Price),
		/// Position closed: (who, position_id, market_price)
		PositionClosed(AccountId, PositionId, Price),
		/// Deposited: (who, pool_id, amount)
		Deposited(AccountId, LiquidityPoolId, Amount),
		/// Withdrew: (who, pool_id, amount)
		Withdrew(AccountId, LiquidityPoolId, Amount),
		/// TraderMarginCalled: (who)
		TraderMarginCalled(AccountId),
		/// TraderBecameSafe: (who)
		TraderBecameSafe(AccountId),
		/// TraderStoppedOut: (who)
		TraderStoppedOut(AccountId),
		/// LiquidityPoolMarginCalled: (pool_id)
		LiquidityPoolMarginCalled(LiquidityPoolId),
		/// LiquidityPoolBecameSafe: (pool_id)
		LiquidityPoolBecameSafe(LiquidityPoolId),
		/// LiquidityPoolForceClosed: (pool_id)
		LiquidityPoolForceClosed(LiquidityPoolId),
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
		PositionNotAllowed,
		CannotOpenPosition,
		CannotOpenMorePosition,
		InsufficientFreeMargin,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = SimpleDispatchInfo::FixedNormal(20_000)]
		pub fn open_position(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			pair: TradingPair,
			leverage: Leverage,
			#[compact] leveraged_amount: Balance,
			#[compact] price: Price,
		) {
			let who = ensure_signed(origin)?;
			Self::_open_position(&who, pool_id, pair, leverage, leveraged_amount, price)?;
		}

		#[weight = SimpleDispatchInfo::FixedNormal(20_000)]
		pub fn close_position(origin, #[compact] position_id: PositionId, #[compact] price: Price) {
			let who = ensure_signed(origin)?;
			Self::_close_position(&who, position_id, Some(price))?;
		}

		#[weight = SimpleDispatchInfo::FixedOperational(10_000)]
		pub fn deposit(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_deposit(&who, pool_id, amount)?;

			Self::deposit_event(RawEvent::Deposited(who, pool_id, amount));
		}

		#[weight = SimpleDispatchInfo::FixedNormal(10_000)]
		pub fn withdraw(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_withdraw(&who, pool_id, amount)?;

			Self::deposit_event(RawEvent::Withdrew(who, pool_id, amount));
		}

		#[weight = SimpleDispatchInfo::FixedOperational(20_000)]
		pub fn trader_margin_call(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			#[compact] pool_id: LiquidityPoolId
		) {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::_trader_margin_call(&who, pool_id)?;
			Self::deposit_event(RawEvent::TraderMarginCalled(who));
		}

		#[weight = SimpleDispatchInfo::FixedNormal(20_000)]
		pub fn trader_become_safe(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			#[compact] pool_id: LiquidityPoolId
		) {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::_trader_become_safe(&who, pool_id)?;
			Self::deposit_event(RawEvent::TraderBecameSafe(who));
		}

		#[weight = SimpleDispatchInfo::FixedOperational(30_000)]
		pub fn trader_stop_out(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			#[compact] pool_id: LiquidityPoolId
		) {
			ensure_none(origin)?;
			let who = T::Lookup::lookup(who)?;

			Self::_trader_stop_out(&who, pool_id)?;
			Self::deposit_event(RawEvent::TraderStoppedOut(who));
		}

		#[weight = SimpleDispatchInfo::FixedOperational(20_000)]
		pub fn liquidity_pool_margin_call(origin, #[compact] pool: LiquidityPoolId) {
			ensure_none(origin)?;
			Self::_liquidity_pool_margin_call(pool)?;
			Self::deposit_event(RawEvent::LiquidityPoolMarginCalled(pool));
		}

		#[weight = SimpleDispatchInfo::FixedNormal(20_000)]
		pub fn liquidity_pool_become_safe(origin, #[compact] pool: LiquidityPoolId) {
			ensure_none(origin)?;
			Self::_liquidity_pool_become_safe(pool)?;
			Self::deposit_event(RawEvent::LiquidityPoolBecameSafe(pool));
		}

		#[weight = SimpleDispatchInfo::FixedOperational(30_000)]
		pub fn liquidity_pool_force_close(origin, #[compact] pool: LiquidityPoolId) {
			ensure_none(origin)?;
			Self::_liquidity_pool_force_close(pool)?;
			Self::deposit_event(RawEvent::LiquidityPoolForceClosed(pool));
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
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		leverage: Leverage,
		leveraged_amount: Balance,
		price: Price,
	) -> DispatchResult {
		Self::_ensure_can_open_more_position(who, pool_id, pair)?;
		ensure!(
			Self::margin_called_traders(who, pool_id).is_none(),
			Error::<T>::MarginCalledTrader
		);
		ensure!(
			Self::margin_called_pools(pool_id).is_none(),
			Error::<T>::MarginCalledPool
		);
		ensure!(
			T::LiquidityPools::is_allowed_position(pool_id, pair, leverage),
			Error::<T>::PositionNotAllowed
		);

		let (held_signum, debit_signum): (i128, i128) = if leverage.is_long() { (1, -1) } else { (-1, 1) };
		let leveraged_held = fixed_128_from_u128(leveraged_amount);
		let debits_price = {
			if leverage.is_long() {
				Self::_ask_price(pool_id, pair, Some(price))?
			} else {
				Self::_bid_price(pool_id, pair, Some(price))?
			}
		};
		let leveraged_debits = leveraged_held
			.checked_mul(&debits_price)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let leveraged_held_in_usd = Self::_usd_value(pair.quote, leveraged_debits)?;
		ensure!(
			T::LiquidityPools::can_open_position(pool_id, pair, leverage, u128_from_fixed_128(leveraged_held_in_usd)),
			Error::<T>::CannotOpenPosition
		);

		let margin_held = {
			let leverage_value = Fixed128::from_natural(leverage.value().into());
			leveraged_held_in_usd
				.checked_div(&leverage_value)
				.expect("leveraged value cannot be zero; qed")
		};
		let open_accumulated_swap_rate =
			T::LiquidityPools::get_accumulated_swap_rate(pool_id, pair, leverage.is_long());
		let position: Position<T> = Position {
			owner: who.clone(),
			pool: pool_id,
			pair,
			leverage,
			leveraged_held: fixed_128_mul_signum(leveraged_held, held_signum),
			leveraged_debits: fixed_128_mul_signum(leveraged_debits, debit_signum),
			leveraged_debits_in_usd: fixed_128_mul_signum(leveraged_held_in_usd, debit_signum),
			open_accumulated_swap_rate,
			margin_held,
		};

		let free_margin = Self::free_margin(who, pool_id)?;
		ensure!(free_margin >= margin_held, Error::<T>::InsufficientFreeMargin);
		Self::_ensure_pool_safe(pool_id, Action::OpenPosition(position.clone()))?;

		let id = Self::_insert_position(who, pool_id, pair, position)?;

		Self::deposit_event(RawEvent::PositionOpened(
			who.clone(),
			id,
			pool_id,
			pair,
			leverage,
			leveraged_amount,
			FixedU128::from_parts(u128_from_fixed_128(debits_price)),
		));

		Ok(())
	}

	fn _close_position(who: &T::AccountId, position_id: PositionId, price: Option<Price>) -> DispatchResult {
		let position = Self::positions(position_id).ok_or(Error::<T>::PositionNotFound)?;
		ensure!(
			<PositionsByTrader<T>>::contains_key(who, (position.pool, position_id)),
			Error::<T>::PositionNotOpenedByTrader
		);
		let (unrealized_pl, market_price) = Self::_unrealized_pl_and_market_price_of_position(&position, price)?;
		let accumulated_swap_rate = Self::_accumulated_swap_rate_of_position(&position)?;
		let balance_delta = unrealized_pl
			.checked_add(&accumulated_swap_rate)
			.ok_or(Error::<T>::NumOutOfBound)?;

		// realizing
		if balance_delta.is_positive() {
			// trader has profit, max realizable is the pool's liquidity
			let pool_liquidity = fixed_128_from_u128(<T::LiquidityPools as LiquidityPools<T::AccountId>>::liquidity(
				position.pool,
			));
			let realized = cmp::min(pool_liquidity, balance_delta);
			<T::LiquidityPools as LiquidityPools<T::AccountId>>::withdraw_liquidity(
				&Self::account_id(),
				position.pool,
				u128_from_fixed_128(realized),
			)?;
			Self::_update_balance(who, position.pool, realized);
		} else {
			// trader has loss, max realizable is the trader's equity
			let equity = Self::equity_of_trader(who, position.pool)?;
			let balance_delta_abs = balance_delta.saturating_abs();
			let realizable = equity.saturating_add(balance_delta_abs);

			// pool get nothing if no realizable from traders
			if realizable.is_positive() {
				let realized = cmp::min(realizable, balance_delta_abs);
				<T::LiquidityPools as LiquidityPools<T::AccountId>>::deposit_liquidity(
					&Self::account_id(),
					position.pool,
					u128_from_fixed_128(realized),
				)?;
			}

			Self::_update_balance(who, position.pool, balance_delta);
		}

		// remove position
		<Positions<T>>::remove(position_id);
		<PositionsByTrader<T>>::remove(who, (position.pool, position_id));
		PositionsByPool::remove(position.pool, (position.pair, position_id));

		Self::deposit_event(RawEvent::PositionClosed(
			who.clone(),
			position_id,
			FixedU128::from_parts(u128_from_fixed_128(market_price)),
		));

		Ok(())
	}

	fn _deposit(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		T::MultiCurrency::transfer(CurrencyId::AUSD, who, &Self::account_id(), amount)?;
		Self::_update_balance(who, pool_id, fixed_128_from_u128(amount));

		Ok(())
	}

	fn _withdraw(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		let free_margin = Self::free_margin(who, pool_id)?;
		let amount_fixed128 = fixed_128_from_u128(amount);
		ensure!(free_margin >= amount_fixed128, Error::<T>::InsufficientFreeMargin);

		T::MultiCurrency::transfer(CurrencyId::AUSD, &Self::account_id(), who, amount)?;
		Self::_update_balance(who, pool_id, fixed_128_mul_signum(amount_fixed128, -1));

		Ok(())
	}

	fn _trader_margin_call(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		if !Self::_is_trader_margin_called(who, pool_id) {
			if Self::_ensure_trader_safe(who, pool_id).is_err() {
				<MarginCalledTraders<T>>::insert(who, pool_id, true);
			} else {
				return Err(Error::<T>::SafeTrader.into());
			}
		}
		Ok(())
	}

	fn _trader_become_safe(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		if Self::_is_trader_margin_called(who, pool_id) {
			if Self::_ensure_trader_safe(who, pool_id).is_ok() {
				<MarginCalledTraders<T>>::remove(who, pool_id);
			} else {
				return Err(Error::<T>::UnsafeTrader.into());
			}
		}
		Ok(())
	}

	fn _trader_stop_out(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		let risk = Self::_check_trader(who, pool_id)?;
		match risk {
			Risk::StopOut => {
				// To stop out a trader:
				//   1. Close the position with the biggest loss.
				//   2. Repeat step 1 until no stop out risk, or all positions of this trader has been closed.

				let mut positions: Vec<(PositionId, Fixed128)> = <PositionsByTrader<T>>::iter(who)
					.filter_map(|((_, position_id), _)| {
						let position = Self::positions(position_id)?;
						if position.pool != pool_id {
							return None;
						}

						let unrealized_pl = Self::_unrealized_pl_of_position(&position).ok()?;
						let accumulated_swap_rate = Self::_accumulated_swap_rate_of_position(&position).ok()?;
						let unrealized = unrealized_pl.checked_add(&accumulated_swap_rate)?;
						Some((position_id, unrealized))
					})
					.collect();
				positions.sort_unstable_by(|x, y| x.1.cmp(&y.1));

				for (id, _) in positions {
					let _ = Self::_close_position(who, id, None);
					let new_risk = Self::_check_trader(who, pool_id)?;
					match new_risk {
						Risk::StopOut => {}
						_ => break,
					}
				}

				if Self::_ensure_trader_safe(who, pool_id).is_ok() && Self::_is_trader_margin_called(who, pool_id) {
					<MarginCalledTraders<T>>::remove(who, pool_id);
				}
				Ok(())
			}
			_ => Err(Error::<T>::NotReachedRiskThreshold.into()),
		}
	}

	fn _liquidity_pool_margin_call(pool: LiquidityPoolId) -> DispatchResult {
		if !Self::_is_pool_margin_called(&pool) {
			if Self::_ensure_pool_safe(pool, Action::None).is_err() {
				MarginCalledPools::insert(pool, true);
			} else {
				return Err(Error::<T>::SafePool.into());
			}
		}
		Ok(())
	}

	fn _liquidity_pool_become_safe(pool: LiquidityPoolId) -> DispatchResult {
		if Self::_is_pool_margin_called(&pool) {
			if Self::_ensure_pool_safe(pool, Action::None).is_ok() {
				MarginCalledPools::remove(pool);
			} else {
				return Err(Error::<T>::UnsafePool.into());
			}
		}
		Ok(())
	}

	fn _liquidity_pool_force_close(pool: LiquidityPoolId) -> DispatchResult {
		match Self::_check_pool(pool, Action::None) {
			Ok(Risk::StopOut) => {
				PositionsByPool::iter(pool).for_each(|((_, position_id), _)| {
					let _ = Self::_liquidity_pool_close_position(pool, position_id);
				});

				if Self::_ensure_pool_safe(pool, Action::None).is_ok() && Self::_is_pool_margin_called(&pool) {
					MarginCalledPools::remove(pool);
				}
				Ok(())
			}
			_ => Err(Error::<T>::NotReachedRiskThreshold.into()),
		}
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
	) -> result::Result<PositionId, DispatchError> {
		let id = Self::next_position_id();
		ensure!(id != PositionId::max_value(), Error::<T>::NoAvailablePositionId);
		NextPositionId::mutate(|id| *id += 1);

		<Positions<T>>::insert(id, position);
		<PositionsByTrader<T>>::insert(who, (pool, id), true);
		PositionsByPool::insert(pool, (pair, id), true);

		Ok(id)
	}

	/// Update `who` balance in `pool_id` by `amount`.
	///
	/// Note this function guarantees op, don't use in possible no-op scenario.
	fn _update_balance(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Fixed128) {
		let new_balance = Self::balances(who, pool_id).saturating_add(amount);
		<Balances<T>>::insert(who, pool_id, new_balance);
	}

	fn _ensure_can_open_more_position(who: &T::AccountId, pool: LiquidityPoolId, pair: TradingPair) -> DispatchResult {
		let count = PositionsByPool::iter(pool).filter(|((p, _), _)| *p == pair).count();
		ensure!(
			count < T::GetPoolMaxOpenPositions::get(),
			Error::<T>::CannotOpenMorePosition
		);
		let count = <PositionsByTrader<T>>::iter(who)
			.filter(|((p, _), _)| *p == pool)
			.count();
		ensure!(
			count < T::GetTraderMaxOpenPositions::get(),
			Error::<T>::CannotOpenMorePosition
		);
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

	/// ask_price = price + ask_spread
	fn _ask_price(pool: LiquidityPoolId, pair: TradingPair, max: Option<Price>) -> Fixed128Result {
		let price = Self::_price(pair.base, pair.quote)?;
		let spread = T::LiquidityPools::get_ask_spread(pool, pair)
			.ok_or(Error::<T>::NoAskSpread)
			.map(Price::from_parts)?;
		let ask_price: Price = price.saturating_add(spread);

		if let Some(m) = max {
			if ask_price > m {
				return Err(Error::<T>::MarketPriceTooHigh.into());
			}
		}

		Ok(fixed_128_from_fixed_u128(ask_price))
	}

	/// bid_price = price - bid_spread
	fn _bid_price(pool: LiquidityPoolId, pair: TradingPair, min: Option<Price>) -> Fixed128Result {
		let price = Self::_price(pair.base, pair.quote)?;
		let spread = T::LiquidityPools::get_bid_spread(pool, pair)
			.ok_or(Error::<T>::NoBidSpread)
			.map(Price::from_parts)?;
		let bid_price = price.saturating_sub(spread);

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
			let p = Self::_price(currency_id, CurrencyId::AUSD)?;
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
		let usd_value = Self::_usd_value(position.pair.quote, unrealized)?;

		Ok((usd_value, curr_price))
	}

	/// Unrealized profit and loss of a given trader in a pool(USD value). It is the sum of unrealized profit and loss
	/// of all positions opened by a trader.
	pub fn unrealized_pl_of_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> Fixed128Result {
		<PositionsByTrader<T>>::iter(who)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
			.filter(|p| p.pool == pool_id)
			.try_fold(Fixed128::zero(), |acc, p| {
				let unrealized = Self::_unrealized_pl_of_position(&p)?;
				acc.checked_add(&unrealized).ok_or(Error::<T>::NumOutOfBound.into())
			})
	}

	/// Sum of all margin held of a given trader in a pool.
	pub fn margin_held(who: &T::AccountId, pool_id: LiquidityPoolId) -> Fixed128 {
		<PositionsByTrader<T>>::iter(who)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
			.filter(|p| p.pool == pool_id)
			.fold(Fixed128::zero(), |acc, p| {
				acc.checked_add(&p.margin_held)
					.expect("margin held cannot overflow; qed")
			})
	}

	/// Accumulated swap rate of a position(USD value).
	///
	/// accumulated_swap_rate_of_position = (current_accumulated - open_accumulated) * leveraged_held
	fn _accumulated_swap_rate_of_position(position: &Position<T>) -> Fixed128Result {
		let rate =
			T::LiquidityPools::get_accumulated_swap_rate(position.pool, position.pair, position.leverage.is_long())
				.checked_sub(&position.open_accumulated_swap_rate)
				.ok_or(Error::<T>::NumOutOfBound)?;
		position
			.leveraged_held
			.checked_mul(&rate)
			.ok_or(Error::<T>::NumOutOfBound.into())
	}

	/// Accumulated swap of all open positions of a given trader(USD value) in a pool.
	fn _accumulated_swap_rate_of_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> Fixed128Result {
		<PositionsByTrader<T>>::iter(who)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
			.filter(|p| p.pool == pool_id)
			.try_fold(Fixed128::zero(), |acc, p| {
				let rate_of_p = Self::_accumulated_swap_rate_of_position(&p)?;
				acc.checked_add(&rate_of_p).ok_or(Error::<T>::NumOutOfBound.into())
			})
	}

	/// equity_of_trader = balance + unrealized_pl + accumulated_swap_rate
	pub fn equity_of_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> Fixed128Result {
		let unrealized = Self::unrealized_pl_of_trader(who, pool_id)?;
		let with_unrealized = Self::balances(who, pool_id)
			.checked_add(&unrealized)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let accumulated_swap_rate = Self::_accumulated_swap_rate_of_trader(who, pool_id)?;
		with_unrealized
			.checked_add(&accumulated_swap_rate)
			.ok_or(Error::<T>::NumOutOfBound.into())
	}

	/// Free margin of a given trader in a pool.
	pub fn free_margin(who: &T::AccountId, pool_id: LiquidityPoolId) -> Fixed128Result {
		let equity = Self::equity_of_trader(who, pool_id)?;
		let margin_held = Self::margin_held(who, pool_id);
		Ok(equity.saturating_sub(margin_held))
	}

	/// Margin level of a given trader in a pool.
	pub fn margin_level(who: &T::AccountId, pool_id: LiquidityPoolId) -> Fixed128Result {
		let equity = Self::equity_of_trader(who, pool_id)?;
		let leveraged_debits_in_usd = <PositionsByTrader<T>>::iter(who)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
			.filter(|p| p.pool == pool_id)
			.try_fold(Fixed128::zero(), |acc, p| {
				acc.checked_add(&p.leveraged_debits_in_usd.saturating_abs())
					.ok_or(Error::<T>::NumOutOfBound)
			})?;

		Ok(equity
			.checked_div(&leveraged_debits_in_usd)
			// if no leveraged held, margin level is max
			.unwrap_or(Fixed128::max_value()))
	}

	/// Ensure a trader is safe.
	///
	/// Return `Ok` if ensured safe, or `Err` if not.
	fn _ensure_trader_safe(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		let risk = Self::_check_trader(who, pool_id)?;
		match risk {
			Risk::None => Ok(()),
			_ => Err(Error::<T>::UnsafeTrader.into()),
		}
	}

	/// Check trader risk after performing an action.
	///
	/// Return `Ok(Risk)`, or `Err` if check fails.
	fn _check_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> Result<Risk, DispatchError> {
		let margin_level = Self::margin_level(who, pool_id)?;
		let RiskThreshold { stop_out, margin_call } = Self::trader_risk_threshold();

		let risk = if margin_level <= stop_out.into() {
			Risk::StopOut
		} else if margin_level <= margin_call.into() {
			Risk::MarginCall
		} else {
			Risk::None
		};

		Ok(risk)
	}
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
enum Action<T: Trait> {
	None,
	Withdraw(Balance),
	OpenPosition(Position<T>),
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
enum Risk {
	None,
	MarginCall,
	StopOut,
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
		let unrealized_pl_and_rate = PositionsByPool::iter(pool)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
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
		let (net, positive, non_positive) = PositionsByPool::iter(pool)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
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

	/// ENP and ELL after performing action.
	///
	/// ENP - Equity to Net Position ratio of a liquidity pool.
	/// ELL - Equity to Longest Leg ratio of a liquidity pool.
	fn _enp_and_ell(pool: LiquidityPoolId, action: Action<T>) -> result::Result<(Fixed128, Fixed128), DispatchError> {
		let equity = Self::_equity_of_pool(pool)?;
		let new_position = match action.clone() {
			Action::OpenPosition(p) => Some(p),
			_ => None,
		};
		let (net_position, longest_leg) = Self::_net_position_and_longest_leg(pool, new_position);

		let equity = match action {
			Action::Withdraw(amount) => equity
				.checked_sub(&fixed_128_from_u128(amount))
				.ok_or(Error::<T>::NumOutOfBound)?,
			_ => equity,
		};

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

	/// Ensure a liquidity pool is safe after performing an action.
	///
	/// Return `Ok` if ensured safe, or `Err` if not.
	fn _ensure_pool_safe(pool: LiquidityPoolId, action: Action<T>) -> DispatchResult {
		match Self::_check_pool(pool, action.clone()) {
			Ok(Risk::None) => Ok(()),
			_ => match action {
				Action::None => Err(Error::<T>::UnsafePool.into()),
				_ => Err(Error::<T>::PoolWouldBeUnsafe.into()),
			},
		}
	}

	/// Check pool risk after performing an action.
	///
	/// Return `Ok(Risk)`, or `Err` if check fails.
	fn _check_pool(pool_id: LiquidityPoolId, action: Action<T>) -> Result<Risk, DispatchError> {
		let enp_threshold = Self::liquidity_pool_enp_threshold();
		let ell_threshold = Self::liquidity_pool_ell_threshold();
		let (enp, ell) = Self::_enp_and_ell(pool_id, action)?;
		if enp <= enp_threshold.stop_out.into() || ell <= ell_threshold.stop_out.into() {
			return Ok(Risk::StopOut);
		} else if enp <= enp_threshold.margin_call.into() || ell <= ell_threshold.margin_call.into() {
			return Ok(Risk::MarginCall);
		}
		Ok(Risk::None)
	}

	/// Force closure position to liquidate liquidity pool based on opened positions.
	///
	/// Return `Ok` if closure success, or `Err` if not.
	fn _liquidity_pool_close_position(pool: LiquidityPoolId, position_id: PositionId) -> DispatchResult {
		let position = Self::positions(position_id).ok_or(Error::<T>::PositionNotFound)?;

		let spread = {
			if position.leverage.is_long() {
				T::LiquidityPools::get_bid_spread(pool, position.pair)
					.ok_or(Error::<T>::NoBidSpread)
					.map(fixed_128_from_u128)?
			} else {
				T::LiquidityPools::get_ask_spread(pool, position.pair)
					.ok_or(Error::<T>::NoAskSpread)
					.map(fixed_128_from_u128)?
			}
		};

		let spread_profit = position
			.leveraged_held
			.saturating_abs()
			.checked_mul(&spread)
			.ok_or(Error::<T>::NumOutOfBound)?;

		let spread_profit_in_usd = Self::_usd_value(position.pair.quote, spread_profit)?;
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

	pub fn enp_and_ell(pool: LiquidityPoolId) -> Option<(Fixed128, Fixed128)> {
		if <T::LiquidityPools as LiquidityPools<T::AccountId>>::pool_exists(pool) {
			let result = Self::_enp_and_ell(pool, Action::None).ok()?;
			return Some(result);
		}
		None
	}

	/// Returns required deposit amount to make pool safe.
	pub fn pool_required_deposit(pool: LiquidityPoolId) -> Option<Fixed128> {
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
		let equity = Self::_equity_of_pool(pool).ok()?;
		let gap = required_equity.checked_sub(&equity)?;

		if gap.is_positive() {
			Some(gap)
		} else {
			Some(Fixed128::zero())
		}
	}
}

impl<T: Trait> LiquidityPoolManager<LiquidityPoolId, Balance> for Module<T> {
	/// Returns if `pool` has liability in margin protocol.
	fn can_remove(pool: LiquidityPoolId) -> bool {
		PositionsByPool::iter(pool).count() == 0
	}

	fn ensure_can_withdraw(pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		Self::_ensure_pool_safe(pool_id, Action::Withdraw(amount))
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
	/// Get a list of `(trader, pool_id)`
	fn _get_traders() -> Vec<(T::AccountId, LiquidityPoolId)> {
		// TODO: use key iter after this gets closed https://github.com/paritytech/substrate/issues/5319
		let mut traders: Vec<(T::AccountId, LiquidityPoolId)> =
			<Positions<T>>::iter().map(|(_, p)| (p.owner, p.pool)).collect();
		traders.sort();
		traders.dedup(); // dedup works as unique for sorted vec, so we sort first
		traders
	}

	/// Get a list of pools
	fn _get_pools() -> Vec<LiquidityPoolId> {
		// TODO: use key iter after this gets closed https://github.com/paritytech/substrate/issues/5319
		let mut pools: Vec<LiquidityPoolId> = <Positions<T>>::iter().map(|(_, p)| p.pool).collect();
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

		for (trader, pool_id) in Self::_get_traders() {
			match Self::_check_trader(&trader, pool_id).map_err(|_| OffchainErr::CheckFail)? {
				Risk::StopOut => {
					let who = T::Lookup::unlookup(trader.clone());
					let call = Call::<T>::trader_stop_out(who, pool_id);
					T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
					debug::native::trace!(
						target: TAG,
						"Trader liquidate [trader = {:?}, block_number = {:?}]",
						trader,
						block_number
					);
				}
				Risk::MarginCall => {
					if !Self::_is_trader_margin_called(&trader, pool_id) {
						let who = T::Lookup::unlookup(trader.clone());
						let call = Call::<T>::trader_margin_call(who, pool_id);
						T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
						debug::native::trace!(
							target: TAG,
							"Trader margin call [trader = {:?}, block_number = {:?}]",
							trader,
							block_number
						);
					}
				}
				Risk::None => {
					if Self::_is_trader_margin_called(&trader, pool_id) {
						let who = T::Lookup::unlookup(trader.clone());
						let call = Call::<T>::trader_become_safe(who, pool_id);
						T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
						debug::native::trace!(
							target: TAG,
							"Trader become safe [trader = {:?}, block_number = {:?}]",
							trader,
							block_number
						);
					}
				}
			}

			Self::_extend_offchain_worker_lock_if_needed();
		}

		for pool_id in Self::_get_pools() {
			match Self::_check_pool(pool_id, Action::None).map_err(|_| OffchainErr::CheckFail)? {
				Risk::StopOut => {
					let call = Call::<T>::liquidity_pool_force_close(pool_id);
					T::SubmitTransaction::submit_unsigned(call).map_err(|_| OffchainErr::SubmitTransaction)?;
					debug::native::trace!(
						target: TAG,
						"Liquidity pool liquidate [pool_id = {:?}, block_number = {:?}]",
						pool_id,
						block_number
					);
				}
				Risk::MarginCall => {
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
				Risk::None => {
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
			}

			Self::_extend_offchain_worker_lock_if_needed();
		}

		Self::_release_offchain_worker_lock();

		debug::native::trace!(target: TAG, "Finished [block_number = {:?}]", block_number);
		Ok(())
	}

	fn _is_trader_margin_called(who: &T::AccountId, pool_id: LiquidityPoolId) -> bool {
		<MarginCalledTraders<T>>::contains_key(&who, pool_id)
	}

	fn _is_pool_margin_called(pool_id: &LiquidityPoolId) -> bool {
		MarginCalledPools::contains_key(pool_id)
	}

	fn _should_stop_out_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> Result<bool, OffchainErr> {
		match Self::_check_trader(who, pool_id).map_err(|_| OffchainErr::CheckFail)? {
			Risk::StopOut => Ok(true),
			_ => Ok(false),
		}
	}

	fn _should_liquidate_pool(pool_id: LiquidityPoolId) -> Result<bool, OffchainErr> {
		match Self::_check_pool(pool_id, Action::None).map_err(|_| OffchainErr::CheckFail)? {
			Risk::StopOut => Ok(true),
			_ => Ok(false),
		}
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

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		match call {
			Call::trader_margin_call(who, pool_id) => {
				let trader = T::Lookup::lookup(who.clone()).expect(InvalidTransaction::Stale.into());
				if Self::_is_trader_margin_called(&trader, *pool_id) {
					return InvalidTransaction::Stale.into();
				}

				ValidTransaction::with_tag_prefix("margin_protocol/trader_margin_call")
					.priority(T::UnsignedPriority::get())
					.and_provides(who)
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			Call::trader_become_safe(who, pool_id) => {
				let trader = T::Lookup::lookup(who.clone()).expect(InvalidTransaction::Stale.into());
				if !Self::_is_trader_margin_called(&trader, *pool_id) {
					return InvalidTransaction::Stale.into();
				}

				ValidTransaction::with_tag_prefix("margin_protocol/trader_become_safe")
					.priority(T::UnsignedPriority::get())
					.and_provides(who)
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			Call::trader_stop_out(who, pool_id) => {
				let trader = T::Lookup::lookup(who.clone()).expect(InvalidTransaction::Stale.into());
				if Self::_should_stop_out_trader(&trader, *pool_id).ok() == Some(true) {
					return ValidTransaction::with_tag_prefix("margin_protocol/trader_stop_out")
						.priority(T::UnsignedPriority::get())
						.and_provides(who)
						.longevity(64_u64)
						.propagate(true)
						.build();
				}
				InvalidTransaction::Stale.into()
			}
			Call::liquidity_pool_margin_call(pool_id) => {
				if Self::_is_pool_margin_called(pool_id) {
					return InvalidTransaction::Stale.into();
				}
				ValidTransaction::with_tag_prefix("margin_protocol/liquidity_pool_margin_call")
					.priority(T::UnsignedPriority::get())
					.and_provides(pool_id)
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			Call::liquidity_pool_become_safe(pool_id) => {
				if !Self::_is_pool_margin_called(pool_id) {
					return InvalidTransaction::Stale.into();
				}

				ValidTransaction::with_tag_prefix("margin_protocol/liquidity_pool_become_safe")
					.priority(T::UnsignedPriority::get())
					.and_provides(pool_id)
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			Call::liquidity_pool_force_close(pool_id) => {
				if Self::_should_liquidate_pool(*pool_id).ok() == Some(true) {
					return ValidTransaction::with_tag_prefix("margin_protocol/liquidity_pool_force_close")
						.priority(T::UnsignedPriority::get())
						.and_provides(pool_id)
						.longevity(64_u64)
						.propagate(true)
						.build();
				}

				InvalidTransaction::Stale.into()
			}
			_ => InvalidTransaction::Call.into(),
		}
	}
}
