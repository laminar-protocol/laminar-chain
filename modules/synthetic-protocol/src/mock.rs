//! Mocks for the synthetic-protocol module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use frame_system as system;
use primitives::H256;
use rstd::marker;
use sr_primitives::{testing::Header, traits::IdentityLookup, Perbill};

use orml_currencies::BasicCurrencyAdapter;
use orml_traits::DataProvider;

use module_primitives::{Balance, BalancePriceConverter, CurrencyId, LiquidityPoolId};
use traits::LiquidityPoolBaseTypes;

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod synthetic_protocol {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		pallet_indices<T>, pallet_balances<T>,
		orml_tokens<T>, orml_currencies<T>,
		synthetic_tokens<T>, synthetic_protocol<T>,
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

type AccountId = u64;
impl frame_system::Trait for Runtime {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sr_primitives::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}
pub type System = system::Module<Runtime>;

impl pallet_indices::Trait for Runtime {
	/// The type for recording indexing into the account enumeration. If this ever overflows, there
	/// will be problems!
	type AccountIndex = u32;
	/// Determine whether an account is dead.
	type IsDeadAccount = Balances;
	/// Use the standard means of resolving an index hint from an id.
	type ResolveHint = pallet_indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
	/// The ubiquitous event type.
	type Event = TestEvent;
}
type Indices = pallet_indices::Module<Runtime>;

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
}

impl pallet_balances::Trait for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// What to do if an account's free balance gets zeroed.
	type OnFreeBalanceZero = ();
	/// What to do if a new account is created.
	type OnNewAccount = Indices;
	type TransferPayment = ();
	type DustRemoval = ();
	/// The ubiquitous event type.
	type Event = TestEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}
type Balances = pallet_balances::Module<Runtime>;

type Amount = i128;
impl orml_tokens::Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
}

parameter_types! {
	pub const GetFlowTokenId: CurrencyId = CurrencyId::FLOW;
}

pub type FlowToken = BasicCurrencyAdapter<Runtime, Balances, Balance, orml_tokens::Error>;

impl orml_currencies::Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = FlowToken;
	type GetNativeCurrencyId = GetFlowTokenId;
}

// TODO: replace this mock
pub struct TestSource;
impl DataProvider<CurrencyId, Price> for TestSource {
	fn get(_currency: &CurrencyId) -> Option<Price> {
		None
	}
}
impl orml_prices::Trait for Runtime {
	type CurrencyId = CurrencyId;
	type Source = TestSource;
}

impl synthetic_tokens::Trait for Runtime {
	type Event = TestEvent;
	type CurrencyId = CurrencyId;
	type Balance = Balance;
	type LiquidityPoolId = LiquidityPoolId;
}
pub type SyntheticTokens = Module<Runtime>;

pub struct TestLiquidityPools<AccountId>(marker::PhantomData<AccountId>);
impl LiquidityPoolBaseTypes for TestLiquidityPools<AccountId> {
	type LiquidityPoolId = LiquidityPoolId;
	type CurrencyId = CurrencyId;
}
impl LiquidityPoolsConfig for TestLiquidityPools<AccountId> {
	fn get_bid_spread(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
		Permill::from_percent(3)
	}

	fn get_ask_spread(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
		Permill::from_percent(3)
	}

	fn get_additional_collateral_ratio(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
		Permill::from_percent(3)
	}
}
impl LiquidityPoolsCurrency<AccountId> for TestLiquidityPools<AccountId> {
	type Balance = Balance;
	type Error = &'static str;

	fn balance(_: Self::LiquidityPoolId) -> Self::Balance {
		Zero::zero()
	}

	fn deposit(_from: &AccountId, _pool_id: Self::LiquidityPoolId, _amount: Self::Balance) -> Result<(), Self::Error> {
		Ok(())
	}

	fn withdraw(_to: &AccountId, _pool_id: Self::LiquidityPoolId, _amount: Self::Balance) -> Result<(), Self::Error> {
		Ok(())
	}
}

parameter_types! {
	pub const GetCollateralCurrencyId: CurrencyId = CurrencyId::AUSD;
}

type CollateralCurrency = orml_currencies::Currency<Runtime, GetCollateralCurrencyId>;
impl Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_currencies::Module<Runtime>;
	type CollateralCurrency = CollateralCurrency;
	type GetCollateralCurrencyId = GetCollateralCurrencyId;
	type PriceProvider = orml_prices::Module<Runtime>;
	type LiquidityPoolsConfig = TestLiquidityPools<AccountId>;
	type LiquidityPoolsCurrency = TestLiquidityPools<AccountId>;
	type BalanceToPrice = BalancePriceConverter;
	type PriceToBalance = BalancePriceConverter;
}

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> runtime_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap()
			.into()
	}
}
