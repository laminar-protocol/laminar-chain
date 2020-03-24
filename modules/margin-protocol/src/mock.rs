//! Mocks for the margin protocol module.

#![cfg(test)]

use frame_support::{impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types};
use frame_system as system;
use primitives::{Balance, CurrencyId, LiquidityPoolId, TradingPair};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::IdentityLookup,
	PerThing, Perbill,
};
use sp_std::{cell::RefCell, collections::btree_map::BTreeMap};
use traits::LiquidityPools;

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

impl_outer_dispatch! {
	pub enum Call for Runtime where origin: Origin {
		margin_protocol::MarginProtocol,
	}
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
	type Call = Call;
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

thread_local! {
	static SPREAD: RefCell<Permill> = RefCell::new(Permill::zero());
	static ACC_SWAP_RATES: RefCell<BTreeMap<TradingPair, Fixed128>> = RefCell::new(BTreeMap::new());
	static LIQUIDITIES: RefCell<BTreeMap<LiquidityPoolId, Balance>> = RefCell::new(BTreeMap::new());
}

pub const MOCK_LIQUIDITY_LOCK_ACCOUNT: u64 = 1000;

pub struct MockLiquidityPools;
impl MockLiquidityPools {
	pub fn spread() -> Permill {
		SPREAD.with(|v| *v.borrow_mut())
	}

	pub fn set_mock_spread(spread: Permill) {
		SPREAD.with(|v| *v.borrow_mut() = spread);
	}

	pub fn accumulated_swap_rate(pair: TradingPair) -> Fixed128 {
		ACC_SWAP_RATES.with(|v| v.borrow_mut().get(&pair).map(|r| *r)).unwrap()
	}

	pub fn set_mock_accumulated_swap_rate(pair: TradingPair, rate: Fixed128) {
		ACC_SWAP_RATES.with(|v| v.borrow_mut().insert(pair, rate));
	}

	pub fn liquidity(pool: LiquidityPoolId) -> Balance {
		LIQUIDITIES.with(|v| v.borrow_mut().get(&pool).map(|l| *l)).unwrap()
	}

	pub fn set_mock_liquidity(pool: LiquidityPoolId, liquidity: Balance) {
		LIQUIDITIES.with(|v| v.borrow_mut().insert(pool, liquidity));
	}
}
impl LiquidityPools<AccountId> for MockLiquidityPools {
	type LiquidityPoolId = LiquidityPoolId;
	type CurrencyId = CurrencyId;
	type Balance = Balance;

	fn ensure_liquidity(_pool_id: Self::LiquidityPoolId, _amount: Self::Balance) -> DispatchResult {
		unimplemented!()
	}

	fn is_owner(_pool_id: Self::LiquidityPoolId, _who: &AccountId) -> bool {
		unimplemented!()
	}

	fn liquidity(pool_id: Self::LiquidityPoolId) -> Self::Balance {
		Self::liquidity(pool_id)
	}

	fn deposit_liquidity(source: &u64, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult {
		<OrmlTokens as MultiCurrency<AccountId>>::transfer(
			CurrencyId::AUSD,
			source,
			&MOCK_LIQUIDITY_LOCK_ACCOUNT,
			amount,
		)?;
		Self::set_mock_liquidity(pool_id, amount + Self::liquidity(pool_id));
		Ok(())
	}

	fn withdraw_liquidity(dest: &u64, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult {
		<OrmlTokens as MultiCurrency<AccountId>>::transfer(
			CurrencyId::AUSD,
			&MOCK_LIQUIDITY_LOCK_ACCOUNT,
			dest,
			amount,
		)?;
		Self::set_mock_liquidity(pool_id, Self::liquidity(pool_id) - amount);
		Ok(())
	}
}

impl MarginProtocolLiquidityPools<AccountId> for MockLiquidityPools {
	type TradingPair = TradingPair;

	fn is_allowed_position(_pool_id: Self::LiquidityPoolId, _pair: Self::TradingPair, _leverage: Leverage) -> bool {
		unimplemented!()
	}

	fn get_bid_spread(_pool_id: Self::LiquidityPoolId, _pair: Self::TradingPair) -> Option<Permill> {
		Some(Self::spread())
	}

	fn get_ask_spread(_pool_id: Self::LiquidityPoolId, _pair: Self::TradingPair) -> Option<Permill> {
		Some(Self::spread())
	}

	fn get_swap_rate(_pool_id: Self::LiquidityPoolId, _pair: Self::TradingPair) -> Fixed128 {
		unimplemented!()
	}

	fn get_accumulated_swap_rate(_pool_id: Self::LiquidityPoolId, pair: Self::TradingPair) -> Fixed128 {
		Self::accumulated_swap_rate(pair)
	}

	fn can_open_position(
		_pool_id: Self::LiquidityPoolId,
		_pair: Self::TradingPair,
		_leverage: Leverage,
		_leveraged_amount: Balance,
	) -> bool {
		unimplemented!()
	}
}

pub struct Treasury;
impl Treasry<AccountId> for Treasury {
	fn account_id() -> AccountId {
		TREASURY_ACCOUNT
	}
}

pub type Extrinsic = TestXt<Call, ()>;
type SubmitTransaction = frame_system::offchain::TransactionSubmitter<(), Call, Extrinsic>;

impl Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = OrmlTokens;
	type LiquidityPools = MockLiquidityPools;
	type PriceProvider = MockPrices;
	type Treasury = Treasury;
	type SubmitTransaction = SubmitTransaction;
	type Call = Call;
}
pub type MarginProtocol = Module<Runtime>;

pub const ALICE: AccountId = 0;
pub const BOB: AccountId = 1;
pub const TREASURY_ACCOUNT: AccountId = 200;
pub const MOCK_POOL: LiquidityPoolId = 100;

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	spread: Permill,
	prices: Vec<(CurrencyId, Price)>,
	swap_rates: Vec<(TradingPair, Fixed128)>,
	trader_risk_threshold: RiskThreshold,
	pool_liquidities: Vec<(LiquidityPoolId, Balance)>,
	liquidity_pool_enp_threshold: RiskThreshold,
	liquidity_pool_ell_threshold: RiskThreshold,
}

impl Default for ExtBuilder {
	/// Spread - 1/1000
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			spread: Permill::from_rational_approximation(1, 1000u32),
			prices: vec![(CurrencyId::AUSD, FixedU128::from_rational(1, 1))],
			swap_rates: vec![],
			trader_risk_threshold: RiskThreshold::default(),
			liquidity_pool_enp_threshold: RiskThreshold::default(),
			liquidity_pool_ell_threshold: RiskThreshold::default(),
			pool_liquidities: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn alice_balance(mut self, balance: Balance) -> Self {
		self.endowed_accounts.push((ALICE, CurrencyId::AUSD, balance));
		self
	}

	pub fn module_balance(mut self, balance: Balance) -> Self {
		self.endowed_accounts
			.push((MarginProtocol::account_id(), CurrencyId::AUSD, balance));
		self
	}

	pub fn spread(mut self, spread: Permill) -> Self {
		self.spread = spread;
		self
	}

	/// `price`: rational(x, y)
	pub fn price(mut self, currency_id: CurrencyId, price: (u128, u128)) -> Self {
		self.prices
			.push((currency_id, FixedU128::from_rational(price.0, price.1)));
		self
	}

	pub fn accumulated_swap_rate(mut self, pair: TradingPair, rate: Fixed128) -> Self {
		self.swap_rates.push((pair, rate));
		self
	}

	pub fn trader_risk_threshold(mut self, threshold: RiskThreshold) -> Self {
		self.trader_risk_threshold = threshold;
		self
	}

	pub fn pool_liquidity(mut self, pool: LiquidityPoolId, liquidity: Balance) -> Self {
		self.pool_liquidities.push((pool, liquidity));
		self.endowed_accounts
			.push((MOCK_LIQUIDITY_LOCK_ACCOUNT, CurrencyId::AUSD, liquidity));
		self
	}

	pub fn liquidity_pool_enp_threshold(mut self, threshold: RiskThreshold) -> Self {
		self.liquidity_pool_enp_threshold = threshold;
		self
	}

	pub fn liquidity_pool_ell_threshold(mut self, threshold: RiskThreshold) -> Self {
		self.liquidity_pool_ell_threshold = threshold;
		self
	}

	fn set_mocks(&self) {
		self.prices
			.iter()
			.for_each(|(c, p)| MockPrices::set_mock_price(*c, Some(*p)));
		MockLiquidityPools::set_mock_spread(self.spread);
		self.swap_rates
			.iter()
			.for_each(|(p, r)| MockLiquidityPools::set_mock_accumulated_swap_rate(*p, *r));
		self.pool_liquidities
			.iter()
			.for_each(|(p, l)| MockLiquidityPools::set_mock_liquidity(*p, *l));
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

		GenesisConfig {
			trader_risk_threshold: self.trader_risk_threshold,
			liquidity_pool_enp_threshold: self.liquidity_pool_enp_threshold,
			liquidity_pool_ell_threshold: self.liquidity_pool_ell_threshold,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
