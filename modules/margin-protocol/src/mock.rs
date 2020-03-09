//! Mocks for the margin protocol module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, ord_parameter_types, parameter_types};
use frame_system as system;
use orml_utilities::Fixed128;
use primitives::{Balance, CurrencyId, LiquidityPoolId};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use sp_std::{cell::RefCell, collections::btree_map::BTreeMap};

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod margin_protocol {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>, orml_tokens<T>, margin_protocol<T>,
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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
}
pub type System = system::Module<Runtime>;

type Amount = i128;

parameter_types! {
	pub const ExistentialDeposit: u128 = 100;
}

impl orml_tokens::Trait for Runtime {
	type Event = TestEvent;
	type Balance = u128;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type ExistentialDeposit = ExistentialDeposit;
	type DustRemoval = ();
}

pub type OrmlTokens = orml_tokens::Module<Runtime>;

thread_local! {
	static PRICES: RefCell<BTreeMap<CurrencyId, Price>> = RefCell::new(BTreeMap::new());
}

pub struct MockPrices;
impl MockPrices {
	pub fn set_mock_price(currency_id: CurrencyId, price: Option<Price>) {
		if let Some(p) = price {
			PRICES.with(|v| v.borrow_mut().insert(currency_id, p));
		} else {
			PRICES.with(|v| v.borrow_mut().remove(&currency_id));
		}
	}

	fn prices(currency_id: CurrencyId) -> Option<Price> {
		PRICES.with(|v| v.borrow_mut().get(&currency_id).map(|p| *p))
	}
}
impl PriceProvider<CurrencyId, Price> for MockPrices {
	fn get_price(base: CurrencyId, quote: CurrencyId) -> Option<Price> {
		let base_price = Self::prices(base)?;
		let quote_price = Self::prices(quote)?;

		quote_price.checked_div(&base_price)
	}
}

//TODO: implementation based on unit test requirements
pub struct MockLiquidityPools;
impl LiquidityPools<AccountId> for MockLiquidityPools {
	type LiquidityPoolId = LiquidityPoolId;
	type CurrencyId = CurrencyId;
	type Balance = Balance;

	fn get_bid_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill> {
		unimplemented!()
	}

	fn get_ask_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill> {
		unimplemented!()
	}

	fn ensure_liquidity(pool_id: Self::LiquidityPoolId) -> bool {
		unimplemented!()
	}

	fn is_owner(pool_id: Self::LiquidityPoolId, who: &u64) -> bool {
		unimplemented!()
	}

	fn is_allowed_position(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId, leverage: Leverage) -> bool {
		unimplemented!()
	}

	fn liquidity(pool_id: Self::LiquidityPoolId) -> Self::Balance {
		unimplemented!()
	}

	fn deposit_liquidity(source: &u64, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult {
		unimplemented!()
	}

	fn withdraw_liquidity(dest: &u64, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult {
		unimplemented!()
	}
}
impl MarginProtocolLiquidityPools<AccountId> for MockLiquidityPools {
	type TradingPair = TradingPairOf<Runtime>;

	fn get_swap_rate(pool_id: Self::LiquidityPoolId, pair: Self::TradingPair) -> Fixed128 {
		unimplemented!()
	}

	fn get_accumulated_swap_rate(pool_id: Self::LiquidityPoolId, pair: Self::TradingPair) -> Fixed128 {
		unimplemented!()
	}

	fn can_open_position(
		pool_id: Self::LiquidityPoolId,
		pair: Self::TradingPair,
		leverage: Leverage,
		leveraged_amount: Self::Balance,
	) -> bool {
		unimplemented!()
	}
}

impl Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = OrmlTokens;
	type LiquidityPools = MockLiquidityPools;
	type PriceProvider = MockPrices;
}

//TODO: more fields based on unit test requirements
pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
