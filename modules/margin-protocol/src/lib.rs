#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use sp_arithmetic::{traits::Saturating, Permill};
use sp_runtime::{traits::StaticLookup, DispatchError, DispatchResult, RuntimeDebug};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, PriceProvider};
use orml_utilities::{Fixed128, FixedU128};
use primitives::{
	arithmetic::{fixed_128_from_fixed_u128, fixed_128_from_u128, u128_from_fixed_128},
	Balance, CurrencyId, Leverage, LiquidityPoolId, Price,
};
use sp_std::{prelude::*, result};
use traits::{LiquidityPoolManager, LiquidityPools, MarginProtocolLiquidityPools};

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

#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq)]
pub struct TradingPair {
	pub base: CurrencyId,
	pub quote: CurrencyId,
}

impl TradingPair {
	fn normalize() {
		// TODO: make the smaller priced currency id as base
		unimplemented!()
	}
}

pub type PositionId = u64;

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq)]
pub struct Position<T: Trait> {
	owner: T::AccountId,
	pool: LiquidityPoolId,
	pair: TradingPair,
	leverage: Leverage,
	leveraged_held: Fixed128,
	leveraged_debits: Fixed128,
	open_accumulated_swap_rate: Fixed128,
	open_margin: Balance,
}

//TODO: set this value
const MAX_POSITIONS_COUNT: u16 = u16::max_value();

#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq)]
pub struct RiskThreshold {
	margin_call: Permill,
	stop_out: Permill,
}

decl_storage! {
	trait Store for Module<T: Trait> as MarginProtocol {
		NextPositionId get(next_position_id): PositionId;
		Positions get(positions): map hasher(blake2_256) PositionId => Option<Position<T>>;
		PositionsByTrader get(positions_by_trader): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) LiquidityPoolId => Vec<PositionId>;
		PositionsByPool get(positions_by_pool): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) (TradingPair, PositionId) => Option<()>;
		// SwapPeriods get(swap_periods): map hasher(black2_256) TradingPair => Option<SwapPeriod>;
		Balances get(balances): map hasher(blake2_256) T::AccountId => Balance;
		MinLiquidationPercent get(min_liquidation_percent): map hasher(blake2_256) TradingPair => Fixed128;
		MarginCalledTraders get(margin_called_traders): map hasher(blake2_256) T::AccountId => Option<()>;
		MarginCalledLiquidityPools get(margin_called_liquidity_pools): map hasher(blake2_256) LiquidityPoolId => Option<()>;
		TraderRiskThreshold get(trader_risk_threshold): map hasher(blake2_256) TradingPair => Option<RiskThreshold>;
		LiquidityPoolENPThreshold get(liquidity_pool_enp_threshold): map hasher(blake2_256) TradingPair => Option<RiskThreshold>;
		LiquidityPoolELLThreshold get(liquidity_pool_ell_threshold): map hasher(blake2_256) TradingPair => Option<RiskThreshold>;
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
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		NoPrice,
		NoAskSpread,
		MarketPriceTooHigh,
		NumOverflow,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		pub fn open_position(origin, pool: LiquidityPoolId, pair: TradingPair, leverage: Leverage, #[compact] leveraged_amount: Balance, price: Price) {
			let who = ensure_signed(origin)?;
			Self::_open_position(&who, pool, pair, leverage, leveraged_amount, price)?;

			Self::deposit_event(RawEvent::PositionOpened(who, pool, pair, leverage, leveraged_amount, price));
		}

		pub fn close_position(origin, position_id: PositionId, price: Price) {
			let who = ensure_signed(origin)?;
			Self::_close_position(&who, position_id, price)?;

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

		// TODO: implementations
		pub fn trader_margin_call(origin, who: <T::Lookup as StaticLookup>::Source) {}
		pub fn trader_become_safe(origin, who: <T::Lookup as StaticLookup>::Source) {}
		pub fn trader_liquidate(origin, who: <T::Lookup as StaticLookup>::Source) {}
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
		// TODO: implementation
		unimplemented!()
	}

	fn _close_position(who: &T::AccountId, position_id: PositionId, price: Price) -> DispatchResult {
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
	fn _ask_price(pool: LiquidityPoolId, held: CurrencyId, debit: CurrencyId, max: Option<Price>) -> PriceResult {
		let price = Self::_price(debit, held)?;
		//FIXME: liquidity pools should provide spread based on trading pair
		let spread: Price = T::LiquidityPools::get_ask_spread(pool, held)
			.ok_or(Error::<T>::NoAskSpread)?
			.into();
		let ask_price: Price = Price::from_natural(1).saturating_add(spread).saturating_mul(price);

		if let Some(m) = max {
			if ask_price > m {
				return Err(Error::<T>::MarketPriceTooHigh.into());
			}
		}

		Ok(ask_price)
	}

	/// bid_price = price * (1 - bid_spread)
	fn _bid_price(pool: LiquidityPoolId, held: CurrencyId, debit: CurrencyId) -> PriceResult {
		let price = Self::_price(debit, held)?;
		//FIXME: liquidity pools should provide spread based on trading pair
		let spread: Price = T::LiquidityPools::get_bid_spread(pool, held)
			.ok_or(Error::<T>::NoAskSpread)?
			.into();

		Ok(Price::from_natural(1).saturating_sub(spread).saturating_mul(price))
	}
}

// Trader helpers
impl<T: Trait> Module<T> {
	/// Unrealized profit and loss of a position.
	///
	/// unrealized_pl_of_position = (curr_price - open_price) * leveraged_held
	fn _unrealized_pl_of_position(position: &Position<T>) -> Fixed128Result {
		// open_price = leveraged_debits / leveraged_held
		let open_price = position
			.leveraged_debits
			.checked_div(&position.leveraged_held)
			.expect("ensured safe on open position")
			.saturating_abs();
		let curr_price = {
			let p = Self::_bid_price(position.pool, position.pair.quote, position.pair.base)?;
			fixed_128_from_fixed_u128(p)
		};
		let price_delta = curr_price.saturating_sub(open_price);

		Ok(position.leveraged_held.saturating_mul(price_delta))
	}

	/// Unrealized profit and loss of a given trader. It is the sum of unrealized profit and loss of all positions
	/// opened by a trader.
	fn _unrealized_pl_of_trader(who: &T::AccountId) -> Fixed128Result {
		<PositionsByTrader<T>>::iter_prefix(who)
			.flatten()
			.filter_map(|position_id| Self::positions(position_id))
			.try_fold(Fixed128::zero(), |acc, p| {
				let unrealized = Self::_unrealized_pl_of_position(&p)?;
				Ok(acc.saturating_add(unrealized))
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
	/// free_balance = balance - margin_held
	fn _free_balance(who: &T::AccountId) -> Balance {
		Self::balances(who).saturating_sub(Self::_margin_held(who))
	}

	/// Free margin: the margin available for opening new positions.
	///
	/// free_margin = balance + unrealized_pl - margin_held
	fn _free_margin(who: &T::AccountId) -> BalanceResult {
		let unrealized = Self::_unrealized_pl_of_trader(who)?;
		Ok(Self::balances(who)
			.saturating_add(u128_from_fixed_128(unrealized))
			.saturating_sub(Self::_margin_held(who)))
	}

	/// Accumulated swap of all open positions of a given trader.
	fn _accumulated_swap_rate(who: &T::AccountId) -> Fixed128 {
		<PositionsByTrader<T>>::iter_prefix(who)
			.flatten()
			.filter_map(|position_id| Self::positions(position_id))
			.fold(Fixed128::zero(), |acc, p| {
				let swap_rate = T::LiquidityPools::get_accumulated_swap_rate(p.pool, p.pair)
					.saturating_sub(p.open_accumulated_swap_rate);
				let swap_fee = p.leveraged_held.saturating_abs().saturating_mul(swap_rate);
				acc.saturating_add(swap_fee)
			})
	}

	/// equity = balance + unrealized_pl + accumulated swap
	fn _equity(who: &T::AccountId) -> Fixed128Result {
		let unrealized = Self::_unrealized_pl_of_trader(who)?;
		Ok(fixed_128_from_u128(Self::balances(who))
			.saturating_add(unrealized)
			.saturating_add(Self::_accumulated_swap_rate(who)))
	}

	/// Margin level of a given user. If `new_position` is `None`, return the margin level based on current positions,
	/// else based on current positions plus this new one.
	fn _margin_level(who: &T::AccountId, new_position: Option<Position<T>>) -> FixedU128 {
		unimplemented!()
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
