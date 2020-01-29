//! Mocks for the synthetic-protocol module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, DispatchResult, Perbill};
use sp_std::{cell::RefCell, collections::btree_map::BTreeMap};

use orml_currencies::Currency;

use module_primitives::{BalancePriceConverter, LiquidityPoolId};
use module_traits::LiquidityPools;

use super::*;

pub use module_primitives::{Balance, CurrencyId, Leverage};

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod synthetic_protocol {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		orml_tokens<T>, orml_currencies<T>,
		module_synthetic_tokens<T>, synthetic_protocol<T>,
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

type Amount = i128;

parameter_types! {
	pub const ExistentialDeposit: u128 = 0;
}

impl orml_tokens::Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type ExistentialDeposit = ();
	type DustRemoval = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::LAMI;
}

type NativeCurrency = Currency<Runtime, GetNativeCurrencyId>;

impl orml_currencies::Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
}

parameter_types! {
	pub const GetCollateralCurrencyId: CurrencyId = CurrencyId::AUSD;
	pub const GetSyntheticCurrencyId: CurrencyId = CurrencyId::FEUR;
}

pub type CollateralCurrency = orml_currencies::Currency<Runtime, GetCollateralCurrencyId>;
pub type SyntheticCurrency = orml_currencies::Currency<Runtime, GetSyntheticCurrencyId>;

impl module_synthetic_tokens::Trait for Runtime {
	type Event = TestEvent;
	type CurrencyId = CurrencyId;
	type Balance = Balance;
	type LiquidityPoolId = LiquidityPoolId;
}
pub type SyntheticTokens = module_synthetic_tokens::Module<Runtime>;

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

thread_local! {
	static SPREAD: RefCell<Permill> = RefCell::new(Permill::zero());
	static ADDITIONAL_COLLATERAL_RATIO: RefCell<Permill> = RefCell::new(Permill::zero());
}

pub struct MockLiquidityPools;
impl MockLiquidityPools {
	fn spread() -> Permill {
		SPREAD.with(|v| *v.borrow_mut())
	}

	fn additional_collateral_ratio() -> Permill {
		ADDITIONAL_COLLATERAL_RATIO.with(|v| *v.borrow_mut())
	}

	pub fn set_mock_spread(spread: Permill) {
		SPREAD.with(|v| *v.borrow_mut() = spread);
	}

	pub fn set_mock_additional_collateral_ratio(ratio: Permill) {
		ADDITIONAL_COLLATERAL_RATIO.with(|v| *v.borrow_mut() = ratio);
	}
}

impl LiquidityPools<AccountId> for MockLiquidityPools {
	type LiquidityPoolId = AccountId;
	type CurrencyId = CurrencyId;
	type Balance = Balance;

	fn get_bid_spread(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Option<Permill> {
		Some(Self::spread())
	}

	fn get_ask_spread(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Option<Permill> {
		Some(Self::spread())
	}

	fn get_additional_collateral_ratio(
		_pool_id: Self::LiquidityPoolId,
		_currency_id: Self::CurrencyId,
	) -> Option<Permill> {
		Some(Self::additional_collateral_ratio())
	}

	/// ALICE is the mock owner
	fn is_owner(_pool_id: Self::LiquidityPoolId, who: &u32) -> bool {
		who == &ALICE
	}

	fn is_allowed_position(
		_pool_id: Self::LiquidityPoolId,
		_currency_id: Self::CurrencyId,
		_leverage: Leverage,
	) -> bool {
		true
	}

	fn liquidity(pool_id: Self::LiquidityPoolId) -> Self::Balance {
		CollateralCurrency::balance(&pool_id)
	}

	fn deposit_liquidity(from: &AccountId, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult {
		CollateralCurrency::transfer(from, &pool_id, amount).map_err(|e| e.into())
	}

	fn withdraw_liquidity(to: &AccountId, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult {
		CollateralCurrency::transfer(&pool_id, to, amount).map_err(|e| e.into())
	}
}

impl Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_currencies::Module<Runtime>;
	type CollateralCurrency = CollateralCurrency;
	type GetCollateralCurrencyId = GetCollateralCurrencyId;
	type PriceProvider = MockPrices;
	type LiquidityPools = MockLiquidityPools;
	type BalanceToPrice = BalancePriceConverter;
	type PriceToBalance = BalancePriceConverter;
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
	spread: Permill,
	additional_collateral_ratio: Permill,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			// collateral price set to `1` for calculation simplicity.
			prices: vec![(CurrencyId::AUSD, FixedU128::from_rational(1, 1))],
			spread: Permill::zero(),
			additional_collateral_ratio: Permill::zero(),
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
		self.synthetic_price(Price::from_rational(3, 1))
	}

	pub fn spread(mut self, spread: Permill) -> Self {
		self.spread = spread;
		self
	}

	pub fn one_percent_spread(self) -> Self {
		self.spread(Permill::from_percent(1))
	}

	pub fn additional_collateral_ratio(mut self, ratio: Permill) -> Self {
		self.additional_collateral_ratio = ratio;
		self
	}

	pub fn ten_percent_additional_collateral_ratio(self) -> Self {
		self.additional_collateral_ratio(Permill::from_percent(10))
	}

	fn set_mocks(&self) {
		self.prices
			.iter()
			.for_each(|(c, p)| MockPrices::set_mock_price(*c, Some(*p)));

		MockLiquidityPools::set_mock_spread(self.spread);
		MockLiquidityPools::set_mock_additional_collateral_ratio(self.additional_collateral_ratio);
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

		t.into()
	}
}
