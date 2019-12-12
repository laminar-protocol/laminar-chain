#![cfg(test)]

use super::*;

use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

use orml_currencies::Currency;
use primitives::{Balance, CurrencyId, LiquidityPoolId};

pub type AccountId = u32;

impl_outer_origin! {
	pub enum Origin for Test {}
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 50;
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::FLOW;
	pub const GetLiquidityCurrencyId: CurrencyId = CurrencyId::AUSD;
	pub const LiquidityCurrencyIds: Vec<CurrencyId> = vec![CurrencyId::AUSD, CurrencyId::FEUR, CurrencyId::FJPY];
}

type NativeCurrency = Currency<Test, GetNativeCurrencyId>;
pub type LiquidityCurrency = orml_currencies::Currency<Test, GetLiquidityCurrencyId>;

impl orml_currencies::Trait for Test {
	type Event = ();
	type MultiCurrency = orml_tokens::Module<Test>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
}

type Amount = i128;
impl orml_tokens::Trait for Test {
	type Event = ();
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
}

pub struct PoolManager;

impl LiquidityPoolManager<LiquidityPoolId> for PoolManager {
	fn can_remove(_pool_id: LiquidityPoolId) -> bool {
		true
	}
}

impl Trait for Test {
	type Event = ();
	type MultiCurrency = orml_currencies::Module<Test>;
	type LiquidityCurrency = LiquidityCurrency;
	type LiquidityPoolId = LiquidityPoolId;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type PoolManager = PoolManager;
	type ExistentialDeposit = ExistentialDeposit;
	type LiquidityCurrencyIds = LiquidityCurrencyIds;
}
pub type ModuleLiquidityPools = Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities {
	let mut t = system::GenesisConfig::default().build_storage::<Test>().unwrap().into();

	orml_tokens::GenesisConfig::<Test> {
		tokens: vec![CurrencyId::AUSD],
		initial_balance: 100_000,
		endowed_accounts: vec![ALICE, BOB],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
