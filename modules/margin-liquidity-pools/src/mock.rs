#![cfg(test)]

use super::*;

use frame_support::{impl_outer_origin, ord_parameter_types, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	DispatchError, Perbill,
};

use orml_currencies::Currency;
use primitives::{Balance, CurrencyId, LiquidityPoolId};
use system::EnsureSignedBy;

pub type BlockNumber = u64;
pub type AccountId = u32;

ord_parameter_types! {
	pub const One: AccountId = 0;
}

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Runtime`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}

impl system::Trait for Runtime {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = BlockNumber;
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
	type ModuleToIndex = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 50;
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::LAMI;
	pub const GetLiquidityCurrencyId: CurrencyId = CurrencyId::AUSD;
	pub const MaxSwap: SwapRate = SwapRate{long: Fixed128::from_natural(-2), short: Fixed128::from_natural(2)};
}

type NativeCurrency = Currency<Runtime, GetNativeCurrencyId>;
pub type LiquidityCurrency = orml_currencies::Currency<Runtime, GetLiquidityCurrencyId>;

impl orml_currencies::Trait for Runtime {
	type Event = ();
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
}

type Amount = i128;
impl orml_tokens::Trait for Runtime {
	type Event = ();
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type ExistentialDeposit = ExistentialDeposit;
	type DustRemoval = ();
}

pub struct PoolManager;
impl LiquidityPoolManager<LiquidityPoolId, Balance> for PoolManager {
	fn can_remove(_pool_id: LiquidityPoolId) -> bool {
		true
	}

	fn get_required_deposit(_pool: LiquidityPoolId) -> result::Result<Balance, DispatchError> {
		unimplemented!()
	}
	fn ensure_can_withdraw(_pool: LiquidityPoolId, _amount: Balance) -> DispatchResult {
		Ok(())
	}
}

impl Trait for Runtime {
	type Event = ();
	type MultiCurrency = orml_currencies::Module<Runtime>;
	type LiquidityCurrency = LiquidityCurrency;
	type LiquidityPoolId = LiquidityPoolId;
	type PoolManager = PoolManager;
	type ExistentialDeposit = ExistentialDeposit;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
	type MaxSwap = MaxSwap;
}
pub type ModuleLiquidityPools = Module<Runtime>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap()
		.into();

	orml_tokens::GenesisConfig::<Runtime> {
		endowed_accounts: vec![(ALICE, CurrencyId::AUSD, 100_000), (BOB, CurrencyId::AUSD, 100_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
