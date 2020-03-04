#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use sp_arithmetic::Permill;
use sp_runtime::{traits::StaticLookup, DispatchResult, RuntimeDebug};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, PriceProvider};
use orml_utilities::Fixed128;
use primitives::{Leverage, Price};
use traits::{LiquidityPoolManager, LiquidityPools, MarginProtocolLiquidityPools};

mod mock;
mod tests;

type BalanceOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::Balance;
type CurrencyIdOf<T> =
	<<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::CurrencyId;
type LiquidityPoolIdOf<T> =
	<<T as Trait>::LiquidityPools as LiquidityPools<<T as frame_system::Trait>::AccountId>>::LiquidityPoolId;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId>;
	type LiquidityPools: MarginProtocolLiquidityPools<
		Self::AccountId,
		CurrencyId = CurrencyIdOf<Self>,
		Balance = BalanceOf<Self>,
	>;
	type PriceProvider: PriceProvider<CurrencyIdOf<Self>, Price>;
}

#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq)]
pub struct TradingPair<CurrencyId> {
	base: CurrencyId,
	quote: CurrencyId,
}
pub type TradingPairOf<T> = TradingPair<CurrencyIdOf<T>>;

impl<CurrencyId> TradingPair<CurrencyId> {
	fn normalize() {
		// TODO: make the smaller priced currency id as base
		unimplemented!()
	}
}

pub type PositionId = u64;

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq)]
pub struct Position<T: Trait> {
	owner: T::AccountId,
	pool: LiquidityPoolIdOf<T>,
	pair: TradingPairOf<T>,
	leverage: Leverage,
	leveraged_holding: BalanceOf<T>,
	leveraged_debits: BalanceOf<T>,
	open_accumulated_swap_rate: Fixed128,
	open_margin: BalanceOf<T>,
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
		PositionsByUser get(positions_by_user): double_map hasher(blake2_256) T::AccountId, hasher(blake2_256) LiquidityPoolIdOf<T> => Vec<PositionId>;
		PositionsByPool get(positions_by_pool): double_map hasher(blake2_256) LiquidityPoolIdOf<T>, hasher(blake2_256) (TradingPairOf<T>, PositionId) => Option<()>;
		// SwapPeriods get(swap_periods): map hasher(black2_256) TradingPairOf<T> => Option<SwapPeriod>;
		Balances get(balances): map hasher(blake2_256) T::AccountId => BalanceOf<T>;
		MinLiquidationPercent get(min_liquidation_percent): map hasher(blake2_256) TradingPairOf<T> => Fixed128;
		MarginCalledTraders get(margin_called_traders): map hasher(blake2_256) T::AccountId => Option<()>;
		MarginCalledLiquidityPools get(margin_called_liquidity_pools): map hasher(blake2_256) LiquidityPoolIdOf<T> => Option<()>;
		TraderRiskThreshold get(trader_risk_threshold): map hasher(blake2_256) TradingPairOf<T> => Option<RiskThreshold>;
		LiquidityPoolENPThreshold get(liquidity_pool_enp_threshold): map hasher(blake2_256) TradingPairOf<T> => Option<RiskThreshold>;
		LiquidityPoolELLThreshold get(liquidity_pool_ell_threshold): map hasher(blake2_256) TradingPairOf<T> => Option<RiskThreshold>;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		LiquidityPoolId = LiquidityPoolIdOf<T>,
		TradingPair = TradingPairOf<T>,
		Amount = BalanceOf<T>
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
	pub enum Error for Module<T: Trait> {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		pub fn open_position(origin, pool: LiquidityPoolIdOf<T>, pair: TradingPairOf<T>, leverage: Leverage, #[compact] leveraged_amount: BalanceOf<T>, price: Price) {
			let who = ensure_signed(origin)?;
			Self::_open_position(&who, pool, pair, leverage, leveraged_amount, price)?;

			Self::deposit_event(RawEvent::PositionOpened(who, pool, pair, leverage, leveraged_amount, price));
		}

		pub fn close_position(origin, position_id: PositionId, price: Price) {
			let who = ensure_signed(origin)?;
			Self::_close_position(&who, position_id, price)?;

			Self::deposit_event(RawEvent::PositionClosed(who, position_id, price));
		}

		pub fn deposit(origin, #[compact] amount: BalanceOf<T>) {
			let who = ensure_signed(origin)?;
			Self::_deposit(&who, amount)?;

			Self::deposit_event(RawEvent::Deposited(who, amount));
		}

		pub fn withdraw(origin, #[compact] amount: BalanceOf<T>) {
			let who = ensure_signed(origin)?;
			Self::_withdraw(&who, amount)?;

			Self::deposit_event(RawEvent::Withdrew(who, amount));
		}

		// TODO: implementations
		pub fn trader_margin_call(origin, who: <T::Lookup as StaticLookup>::Source) {}
		pub fn trader_become_safe(origin, who: <T::Lookup as StaticLookup>::Source) {}
		pub fn trader_liquidate(origin, who: <T::Lookup as StaticLookup>::Source) {}
		pub fn liquidity_pool_margin_call(origin, pool: LiquidityPoolIdOf<T>) {}
		pub fn liquidity_pool_become_safe(origin, pool: LiquidityPoolIdOf<T>) {}
		pub fn liquidity_pool_liquidate(origin, pool: LiquidityPoolIdOf<T>) {}
	}
}

impl<T: Trait> Module<T> {
	fn _open_position(
		who: &T::AccountId,
		pool: LiquidityPoolIdOf<T>,
		pair: TradingPairOf<T>,
		leverage: Leverage,
		leveraged_amount: BalanceOf<T>,
		price: Price,
	) -> DispatchResult {
		// TODO: implementation
		unimplemented!()
	}

	fn _close_position(who: &T::AccountId, position_id: PositionId, price: Price) -> DispatchResult {
		// TODO: implementation
		unimplemented!()
	}

	fn _deposit(who: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
		// TODO: implementation
		unimplemented!()
	}

	fn _withdraw(who: &T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
		// TODO: implementation
		unimplemented!()
	}
}

//TODO: implementations, prevent open new position for margin called pools
impl<T: Trait> LiquidityPoolManager<LiquidityPoolIdOf<T>, BalanceOf<T>> for Module<T> {
	fn can_remove(pool: LiquidityPoolIdOf<T>) -> bool {
		unimplemented!()
	}

	fn get_required_deposit(pool: LiquidityPoolIdOf<T>) -> BalanceOf<T> {
		unimplemented!()
	}
}
