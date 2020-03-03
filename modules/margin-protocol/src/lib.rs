#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, EncodeLike};
use core::fmt;
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use sp_runtime::RuntimeDebug;
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, PriceProvider};
use orml_utilities::Fixed128;
use primitives::{Leverage, Price};
use traits::{LiquidityPools, MarginProtocolLiquidityPools};

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

#[derive(Encode, Decode, EncodeLike, Copy, Clone, RuntimeDebug, Eq, PartialEq)]
pub struct TradingPair<T: Trait> {
	base: CurrencyIdOf<T>,
	quote: CurrencyIdOf<T>,
}

impl<T: Trait> TradingPair<T> {
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
	pair: TradingPair<T>,
	leverage: Leverage,
	leveraged_holding: BalanceOf<T>,
	leveraged_debits: BalanceOf<T>,
	open_accumulated_swap_rate: Fixed128,
	open_margin: BalanceOf<T>,
}

//TODO: set this value
const MAX_POSITIONS_COUNT: u16 = u16::max_value();

decl_storage! {
	trait Store for Module<T: Trait> as MarginProtocol {
		NextPositionId get(next_position_id): PositionId;
		Positions get(positions): map hasher(blake2_256) PositionId => Option<Position<T>>;
		PositionsByUser get(positions_by_user): double_map hasher(blake2_256) T::AccountId, hasher(blake2_256) LiquidityPoolIdOf<T> => Vec<PositionId>;
		PositionsByPool get(positions_by_pool): double_map hasher(blake2_256) LiquidityPoolIdOf<T>, hasher(blake2_256) (TradingPair<T>, PositionId) => Option<()>;
		// SwapPeriods get(swap_periods): map hasher(black2_256) TradingPair<T> => Option<SwapPeriod>;
		Balances get(balances): map hasher(blake2_256) T::AccountId => BalanceOf<T>;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
	{
		Dummy(AccountId),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		// pub fn open_position(origin, pool: LiquidityPoolIdOf<T>, pair: TradingPair<T>, leverage: Leverage, leveraged_amount: BalanceOf<T>, price: Price) {}
		pub fn test(origin, pair: TradingPair<T>) {}

		pub fn close_position(origin, position_id: PositionId, price: Price) {}

		pub fn deposit(origin, amount: BalanceOf<T>) {}

		pub fn withdraw(origin, amount: BalanceOf<T>) {}
	}
}

impl<T: Trait> Module<T> {}
