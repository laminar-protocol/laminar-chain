#![cfg(test)]

use super::*;

use frame_support::{ord_parameter_types, parameter_types, weights::Weight};
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, Block as BlockT, IdentityLookup},
	Perbill,
};

use primitives::{Balance, CurrencyId, LiquidityPoolId};

pub type BlockNumber = u64;
pub type AccountId = u128;

ord_parameter_types! {
	pub const One: AccountId = 0;
}

pub use crate as base_liquidity_pools;

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Runtime`) which `impl`s each of the
// configuration traits of modules we want to use.
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
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 50;
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::LAMI;
	pub const GetLiquidityCurrencyId: CurrencyId = CurrencyId::AUSD;
	pub const IdentityDeposit: u128 = 1000;
}

impl pallet_balances::Trait for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Module<Runtime>;
}

pub type NativeCurrency = orml_currencies::BasicCurrencyAdapter<Runtime, Balances, Balance>;
pub type LiquidityCurrency = orml_currencies::Currency<Runtime, GetLiquidityCurrencyId>;

impl orml_currencies::Trait for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
}

type Amount = i128;
impl orml_tokens::Trait for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type DustRemoval = ();
	type OnReceived = ();
}

pub struct PoolManager;
impl BaseLiquidityPoolManager<LiquidityPoolId, Balance> for PoolManager {
	fn can_remove(_pool_id: LiquidityPoolId) -> bool {
		true
	}
	fn ensure_can_withdraw(_pool: LiquidityPoolId, _amount: Balance) -> DispatchResult {
		Ok(())
	}
}

pub struct DummyOnDisable;
impl OnDisableLiquidityPool for DummyOnDisable {
	fn on_disable(_: LiquidityPoolId) {}
}

pub struct DummyOnRemove;
impl OnRemoveLiquidityPool for DummyOnRemove {
	fn on_remove(_: LiquidityPoolId) {}
}

parameter_types! {
	pub const Instance1ModuleId: ModuleId = ModuleId(*b"test/lp1");
}

impl Trait for Runtime {
	type Event = Event;
	type LiquidityCurrency = LiquidityCurrency;
	type PoolManager = PoolManager;
	type ExistentialDeposit = ExistentialDeposit;
	type IdentityDeposit = IdentityDeposit;
	type IdentityDepositCurrency = Balances;
	type ModuleId = Instance1ModuleId;
	type OnDisableLiquidityPool = DummyOnDisable;
	type OnRemoveLiquidityPool = DummyOnRemove;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
}

impl Trait<Instance1> for Runtime {
	type Event = Event;
	type LiquidityCurrency = LiquidityCurrency;
	type PoolManager = PoolManager;
	type ExistentialDeposit = ExistentialDeposit;
	type IdentityDeposit = IdentityDeposit;
	type IdentityDepositCurrency = Balances;
	type ModuleId = Instance1ModuleId;
	type OnDisableLiquidityPool = DummyOnDisable;
	type OnRemoveLiquidityPool = DummyOnRemove;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
}
pub type Instance1Module = Module<Runtime, Instance1>;

parameter_types! {
	pub const Instance2ModuleId: ModuleId = ModuleId(*b"test/lp2");
}

impl Trait<Instance2> for Runtime {
	type Event = Event;
	type LiquidityCurrency = LiquidityCurrency;
	type PoolManager = PoolManager;
	type ExistentialDeposit = ExistentialDeposit;
	type IdentityDeposit = IdentityDeposit;
	type IdentityDepositCurrency = Balances;
	type ModuleId = Instance2ModuleId;
	type OnDisableLiquidityPool = DummyOnDisable;
	type OnRemoveLiquidityPool = DummyOnRemove;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
}
pub type Instance2Module = Module<Runtime, Instance2>;

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, Call, ()>;
frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
	NodeBlock = Block,
	UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: system::{Module, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		BaseLiquidityPoolsForMargin: base_liquidity_pools::<Instance1>::{Module, Storage, Call, Event<T>},
		BaseLiquidityPoolsForSynthetic: base_liquidity_pools::<Instance2>::{Module, Storage, Call, Event<T>},
		DefaultBaseLiquidityPools: base_liquidity_pools::{Module, Storage, Call, Event<T>},
	}
);

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap()
		.into();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, 100_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	orml_tokens::GenesisConfig::<Runtime> {
		endowed_accounts: vec![(ALICE, CurrencyId::AUSD, 100_000), (BOB, CurrencyId::AUSD, 100_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
