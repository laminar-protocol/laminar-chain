//! Mocks for the synthetic-protocol module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchResult, Perbill, Permill};
use sp_std::{cell::RefCell, collections::btree_map::BTreeMap};

use orml_currencies::Currency;
use orml_traits::{parameter_type_with_key, DataProvider, DefaultPriceProvider};

use laminar_primitives::LiquidityPoolId;
use module_traits::{LiquidityPools, SyntheticProtocolLiquidityPools};

use super::*;

pub use laminar_primitives::{Balance, CurrencyId, Leverage};

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

ord_parameter_types! {
	pub const One: AccountId = 1;
}

mod synthetic_protocol {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		orml_tokens<T>, orml_currencies<T>,
		module_synthetic_tokens, synthetic_protocol<T>,
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

pub type AccountId = u32;
impl frame_system::Config for Runtime {
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
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type MaximumBlockWeight = MaximumBlockWeight;
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type PalletInfo = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}
pub type System = frame_system::Module<Runtime>;

type Amount = i128;

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		Zero::zero()
	};
}

impl orml_tokens::Config for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = orml_tokens::TransferDust<Runtime, One>;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::LAMI;
}

type NativeCurrency = Currency<Runtime, GetNativeCurrencyId>;

impl orml_currencies::Config for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const GetCollateralCurrencyId: CurrencyId = CurrencyId::AUSD;
	pub const GetSyntheticCurrencyId: CurrencyId = CurrencyId::FEUR;
	pub SyntheticCurrencyIds: Vec<CurrencyId> = vec![CurrencyId::FEUR];
	pub const DefaultExtremeRatio: Permill = Permill::from_percent(1);
	pub const DefaultLiquidationRatio: Permill = Permill::from_percent(5);
	pub const DefaultCollateralRatio: Permill = Permill::from_percent(10);
}

pub type CollateralCurrency = orml_currencies::Currency<Runtime, GetCollateralCurrencyId>;
pub type SyntheticCurrency = orml_currencies::Currency<Runtime, GetSyntheticCurrencyId>;

impl module_synthetic_tokens::Config for Runtime {
	type Event = TestEvent;
	type DefaultExtremeRatio = DefaultExtremeRatio;
	type DefaultLiquidationRatio = DefaultLiquidationRatio;
	type DefaultCollateralRatio = DefaultCollateralRatio;
	type SyntheticCurrencyIds = SyntheticCurrencyIds;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
}
pub type TestSyntheticTokens = module_synthetic_tokens::Module<Runtime>;

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

impl DataProvider<CurrencyId, Price> for MockPrices {
	fn get(key: &CurrencyId) -> Option<Price> {
		Self::prices(*key)
	}
}

thread_local! {
	static SPREAD: RefCell<Price> = RefCell::new(Price::zero());
	static ADDITIONAL_COLLATERAL_RATIO: RefCell<Permill> = RefCell::new(Permill::zero());
	static IS_ALLOWED: RefCell<bool> = RefCell::new(false);
}

pub struct MockLiquidityPools;
impl MockLiquidityPools {
	fn spread() -> Price {
		SPREAD.with(|v| *v.borrow_mut())
	}

	fn additional_collateral_ratio() -> Permill {
		ADDITIONAL_COLLATERAL_RATIO.with(|v| *v.borrow_mut())
	}

	fn is_allowed() -> bool {
		IS_ALLOWED.with(|v| *v.borrow_mut())
	}

	pub fn set_mock_spread(spread: Price) {
		SPREAD.with(|v| *v.borrow_mut() = spread);
	}

	pub fn set_mock_additional_collateral_ratio(ratio: Permill) {
		ADDITIONAL_COLLATERAL_RATIO.with(|v| *v.borrow_mut() = ratio);
	}

	pub fn set_is_allowed(allowed: bool) {
		IS_ALLOWED.with(|v| *v.borrow_mut() = allowed);
	}
}

impl LiquidityPools<AccountId> for MockLiquidityPools {
	fn all() -> Vec<LiquidityPoolId> {
		unimplemented!()
	}

	/// ALICE is the mock owner
	fn is_owner(_pool_id: LiquidityPoolId, who: &u32) -> bool {
		who == &ALICE
	}

	fn pool_exists(pool_id: LiquidityPoolId) -> bool {
		pool_id == MOCK_POOL
	}

	fn liquidity(pool_id: LiquidityPoolId) -> Balance {
		CollateralCurrency::free_balance(&pool_id)
	}

	fn deposit_liquidity(from: &AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		CollateralCurrency::transfer(from, &pool_id, amount).map_err(|e| e.into())
	}

	fn withdraw_liquidity(to: &AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		CollateralCurrency::transfer(&pool_id, to, amount).map_err(|e| e.into())
	}
}

impl SyntheticProtocolLiquidityPools<AccountId> for MockLiquidityPools {
	fn bid_spread(_pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Price> {
		let price = MockPrices::prices(currency_id)?;
		Some(Self::spread().saturating_mul(price))
	}

	fn ask_spread(_pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Price> {
		let price = MockPrices::prices(currency_id)?;
		Some(Self::spread().saturating_mul(price))
	}

	fn additional_collateral_ratio(_pool_id: LiquidityPoolId, _currency_id: CurrencyId) -> Permill {
		Self::additional_collateral_ratio()
	}

	fn can_mint(_pool_id: LiquidityPoolId, _currency_id: CurrencyId) -> bool {
		Self::is_allowed()
	}
}

impl Config for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_currencies::Module<Runtime>;
	type CollateralCurrency = CollateralCurrency;
	type GetCollateralCurrencyId = GetCollateralCurrencyId;
	type PriceProvider = DefaultPriceProvider<CurrencyId, MockPrices>;
	type LiquidityPools = MockLiquidityPools;
	type SyntheticProtocolLiquidityPools = MockLiquidityPools;
	type WeightInfo = ();
}
pub type SyntheticProtocol = Module<Runtime>;

pub const ALICE: AccountId = 0;
pub const BOB: AccountId = 1;
pub fn origin_of(account_id: AccountId) -> Origin {
	Origin::signed(account_id)
}

pub const MOCK_POOL: LiquidityPoolId = 100;
pub const ANOTHER_MOCK_POOL: LiquidityPoolId = 101;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	prices: Vec<(CurrencyId, Price)>,
	spread: Price,
	additional_collateral_ratio: Permill,
	is_allowed: bool,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			// collateral price set to `1` for calculation simplicity.
			prices: vec![(CurrencyId::AUSD, Price::saturating_from_rational(1, 1))],
			spread: Price::zero(),
			additional_collateral_ratio: Permill::zero(),
			is_allowed: true,
		}
	}
}

pub const ONE_MILL: Balance = 1000_000;
impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	// one_million is big enough for testing, considering spread is 0.5% on average, and small enough
	// to do math verification by hand.
	pub fn one_million_for_alice_n_mock_pool(self) -> Self {
		self.balances(vec![
			(ALICE, CurrencyId::AUSD, ONE_MILL),
			(MOCK_POOL, CurrencyId::AUSD, ONE_MILL),
		])
	}

	pub fn synthetic_price(mut self, price: Price) -> Self {
		self.prices.push((CurrencyId::FEUR, price));
		self
	}

	/// set synthetic price to `3`
	pub fn synthetic_price_three(self) -> Self {
		self.synthetic_price(Price::saturating_from_rational(3, 1))
	}

	pub fn spread(mut self, spread: Price) -> Self {
		self.spread = spread;
		self
	}

	pub fn one_percent_spread(self) -> Self {
		self.spread(Price::from_fraction(0.01))
	}

	pub fn additional_collateral_ratio(mut self, ratio: Permill) -> Self {
		self.additional_collateral_ratio = ratio;
		self
	}

	pub fn ten_percent_additional_collateral_ratio(self) -> Self {
		self.additional_collateral_ratio(Permill::from_percent(10))
	}

	pub fn set_is_allowed(mut self, val: bool) -> Self {
		self.is_allowed = val;
		self
	}

	fn set_mocks(&self) {
		self.prices
			.iter()
			.for_each(|(c, p)| MockPrices::set_mock_price(*c, Some(*p)));

		MockLiquidityPools::set_mock_spread(self.spread);
		MockLiquidityPools::set_mock_additional_collateral_ratio(self.additional_collateral_ratio);
		MockLiquidityPools::set_is_allowed(self.is_allowed);
	}

	pub fn build(self) -> sp_io::TestExternalities {
		self.set_mocks();

		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
