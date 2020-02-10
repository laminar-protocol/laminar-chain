//! Mocks for the synthetic-tokens module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, ord_parameter_types, parameter_types};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use system::EnsureSignedBy;

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod synthetic_tokens {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		synthetic_tokens<T>,
	}
}

ord_parameter_types! {
	pub const One: AccountId = 0;
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const SyntheticCurrencyIds: Vec<CurrencyId> = vec![FEUR];
}

type AccountId = u64;
impl frame_system::Trait for Runtime {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
}
pub type System = system::Module<Runtime>;

type CurrencyId = u32;
impl Trait for Runtime {
	type Event = TestEvent;
	type CurrencyId = CurrencyId;
	type Balance = u64;
	type LiquidityPoolId = u32;
	type SyntheticCurrencyIds = SyntheticCurrencyIds;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
}

pub type SyntheticTokens = Module<Runtime>;

pub const FEUR: CurrencyId = 0;

pub const ROOT: Origin = Origin::ROOT;

const ALICE_ACC_ID: AccountId = 0;
pub fn alice() -> Origin {
	Origin::signed(ALICE_ACC_ID)
}

const BOB_ACC_ID: AccountId = 1;
pub fn bob() -> Origin {
	Origin::signed(BOB_ACC_ID)
}

#[derive(Default)]
pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap()
			.into()
	}
}
