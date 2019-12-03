//! Mocks for the synthetic-protocol module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use frame_system as system;
use primitives::H256;
use rstd::marker;
use sr_primitives::{testing::Header, traits::IdentityLookup, Perbill};

use orml_currencies::BasicCurrencyAdapter;
use orml_traits::DataProvider;

use module_primitives::{Balance, BalancePriceConverter, LiquidityPoolId};
use traits::LiquidityPoolBaseTypes;

use super::*;

pub use module_primitives::CurrencyId;

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

pub const DEFAULT_PRICE: (Balance, Balance) = (12, 10);
/// price = x / y
#[derive(Debug)]
pub struct MockPrice(Balance, Balance);
impl MockPrice {
	pub fn get(&self) -> Option<Price> {
		if self.0 == 0 && self.1 == 0 {
			None
		} else {
			Some(Price::from_rational(self.0, self.1))
		}
	}

	pub fn set_price(&mut self, x: Balance, y: Balance) {
		self.0 = x;
		self.1 = y;
	}

	pub fn set_default(&mut self) {
		self.0 = DEFAULT_PRICE.0;
		self.1 = DEFAULT_PRICE.1;
	}

	pub fn set_none(&mut self) {
		self.0 = 0;
		self.1 = 0;
	}
}

pub static mut MOCK_PRICE_SOURCE: MockPrice = MockPrice(DEFAULT_PRICE.0, DEFAULT_PRICE.1);
pub struct TestSource;
impl DataProvider<CurrencyId, Price> for TestSource {
	fn get(currency: &CurrencyId) -> Option<Price> {
		match currency {
			CurrencyId::AUSD => Some(Price::from_rational(1, 1)),
			_ => unsafe { MOCK_PRICE_SOURCE.get() },
		}
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

pub const MOCK_POOL: LiquidityPoolId = 1;

pub struct TestLiquidityPools<AccountId>(marker::PhantomData<AccountId>);
impl LiquidityPoolBaseTypes for TestLiquidityPools<AccountId> {
	type LiquidityPoolId = LiquidityPoolId;
	type CurrencyId = CurrencyId;
}

pub fn spread() -> Permill {
	Permill::from_rational_approximation(5u32, 1000u32)
}
pub fn greedy_slippage() -> Permill {
	Permill::from_rational_approximation(3u32, 1000u32)
}
pub fn tolerable_slippage() -> Permill {
	Permill::from_rational_approximation(7u32, 1000u32)
}
pub fn additional_collateral_ratio() -> Permill {
	Permill::from_percent(5)
}

impl LiquidityPoolsConfig for TestLiquidityPools<AccountId> {
	fn get_bid_spread(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
		spread()
	}

	fn get_ask_spread(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
		spread()
	}

	fn get_additional_collateral_ratio(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
		additional_collateral_ratio()
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
pub type SyntheticProtocol = Module<Runtime>;

const ALICE_ACC_ID: AccountId = 0;
pub fn alice() -> Origin {
	Origin::signed(ALICE_ACC_ID)
}

pub struct ExtBuilder {
	currency_id: CurrencyId,
	endowed_accounts: Vec<AccountId>,
	initial_balance: Balance,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			currency_id: CurrencyId::AUSD,
			endowed_accounts: vec![0],
			initial_balance: 0,
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, account_ids: Vec<AccountId>, initial_balance: Balance) -> Self {
		self.endowed_accounts = account_ids;
		self.initial_balance = initial_balance;
		self
	}

	pub fn one_hundred_usd_for_alice(self) -> Self {
		self.balances(vec![ALICE_ACC_ID], 100)
	}

	pub fn build_and_reset_env(self) -> runtime_io::TestExternalities {
		unsafe {
			MOCK_PRICE_SOURCE.set_default();
		}

		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			tokens: vec![self.currency_id],
			initial_balance: self.initial_balance,
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
