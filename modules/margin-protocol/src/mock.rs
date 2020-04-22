//! Mocks for the margin protocol module.

#![cfg(test)]

use frame_support::{impl_outer_dispatch, impl_outer_event, impl_outer_origin, ord_parameter_types, parameter_types};
use frame_system as system;
use frame_system::EnsureSignedBy;
use orml_prices::DefaultPriceProvider;
use orml_traits::DataProvider;
use primitives::{Balance, CurrencyId, LiquidityPoolId, TradingPair};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::IdentityLookup,
	Perbill,
};
use sp_std::{cell::RefCell, collections::btree_map::BTreeMap};
use traits::LiquidityPools;

use super::*;

ord_parameter_types! {
	pub const One: AccountId = 0;
}

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

impl DataProvider<CurrencyId, Price> for MockPrices {
	fn get(key: &CurrencyId) -> Option<Price> {
		Self::prices(*key)
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
	fn all() -> Vec<LiquidityPoolId> {
		unimplemented!()
	}

	fn is_owner(_pool_id: LiquidityPoolId, _who: &AccountId) -> bool {
		unimplemented!()
	}

	fn pool_exists(pool_id: LiquidityPoolId) -> bool {
		pool_id == MOCK_POOL
	}

	fn liquidity(pool_id: LiquidityPoolId) -> Balance {
		Self::liquidity(pool_id)
	}

	fn deposit_liquidity(source: &u64, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		<OrmlTokens as MultiCurrency<AccountId>>::transfer(
			CurrencyId::AUSD,
			source,
			&MOCK_LIQUIDITY_LOCK_ACCOUNT,
			amount,
		)?;
		Self::set_mock_liquidity(pool_id, amount + Self::liquidity(pool_id));
		Ok(())
	}

	fn withdraw_liquidity(dest: &u64, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
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
	fn is_allowed_position(_pool_id: LiquidityPoolId, _pair: TradingPair, _leverage: Leverage) -> bool {
		true
	}

	fn get_bid_spread(_pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Balance> {
		let base_price = MockPrices::prices(pair.base)?;
		let quote_price = MockPrices::prices(pair.quote)?;
		let price = base_price.checked_div(&quote_price).unwrap();
		Some(Self::spread().mul_ceil(price.deconstruct()))
	}

	fn get_ask_spread(_pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Balance> {
		let base_price = MockPrices::prices(pair.base)?;
		let quote_price = MockPrices::prices(pair.quote)?;
		let price = base_price.checked_div(&quote_price).unwrap();
		Some(Self::spread().mul_ceil(price.deconstruct()))
	}

	fn get_swap_rate(_pool_id: LiquidityPoolId, _pair: TradingPair, _is_long: bool) -> Fixed128 {
		unimplemented!()
	}

	fn get_accumulated_swap_rate(_pool_id: LiquidityPoolId, pair: TradingPair, _is_long: bool) -> Fixed128 {
		Self::accumulated_swap_rate(pair)
	}

	fn can_open_position(
		_pool_id: LiquidityPoolId,
		_pair: TradingPair,
		_leverage: Leverage,
		_leveraged_amount: Balance,
	) -> bool {
		true
	}
}

pub struct MockTreasury;
impl Treasury<AccountId> for MockTreasury {
	fn account_id() -> AccountId {
		TREASURY_ACCOUNT
	}
}

pub type Extrinsic = TestXt<Call, ()>;
type SubmitTransaction = frame_system::offchain::TransactionSubmitter<(), Call, Extrinsic>;

parameter_types! {
	pub const GetTraderMaxOpenPositions: usize = 200;
	pub const GetPoolMaxOpenPositions: usize = 1000;
}

impl Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = OrmlTokens;
	type LiquidityPools = MockLiquidityPools;
	type PriceProvider = DefaultPriceProvider<CurrencyId, MockPrices>;
	type Treasury = MockTreasury;
	type SubmitTransaction = SubmitTransaction;
	type Call = Call;
	type GetTraderMaxOpenPositions = GetTraderMaxOpenPositions;
	type GetPoolMaxOpenPositions = GetPoolMaxOpenPositions;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
}
pub type MarginProtocol = Module<Runtime>;

pub const ALICE: AccountId = 0;
pub const BOB: AccountId = 1;
pub const TREASURY_ACCOUNT: AccountId = 3;
pub const MOCK_POOL: LiquidityPoolId = 100;

pub const EUR_USD_PAIR: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};

pub const JPY_EUR_PAIR: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::FJPY,
};

/// Print status of a trader, only for unit tests debugging purpose.
pub fn print_trader_summary(who: &AccountId, name: Option<&'static str>) {
	println!("------------------------------");
	if let Some(n) = name {
		println!("Name: {:?}", n);
	}
	let position_ids: Vec<PositionId> = <PositionsByTrader<Runtime>>::iter(who)
		.map(|((_, position_id), _)| position_id)
		.collect();
	println!("Positions: {:?}", position_ids);
	println!("Balance: {:?}", MarginProtocol::balances(who));
	println!("Free margin: {:?}", MarginProtocol::free_margin(who));
	println!("Unrealized PL: {:?}", MarginProtocol::unrealized_pl_of_trader(who));
	println!("Equity: {:?}", MarginProtocol::equity_of_trader(who));
	println!("Margin level: {:?}", MarginProtocol::margin_level(who));
	println!("------------------------------");
}

#[allow(dead_code)]
pub fn print_alice_summary() {
	print_trader_summary(&ALICE, Some("Alice"));
}

#[allow(dead_code)]
pub fn print_bob_summary() {
	print_trader_summary(&BOB, Some("Bob"));
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
	spread: Permill,
	prices: Vec<(CurrencyId, Price)>,
	swap_rates: Vec<(TradingPair, Fixed128)>,
	pool_liquidities: Vec<(LiquidityPoolId, Balance)>,
}

impl Default for ExtBuilder {
	/// Spread - 1/1000
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
			spread: Permill::from_rational_approximation(1, 1000u32),
			prices: vec![(CurrencyId::AUSD, FixedU128::from_rational(1, 1))],
			swap_rates: vec![],
			pool_liquidities: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn alice_balance(mut self, balance: Balance) -> Self {
		self.endowed_accounts.push((ALICE, CurrencyId::AUSD, balance));
		self
	}

	pub fn module_balance(mut self, balance: Fixed128) -> Self {
		self.endowed_accounts.push((
			MarginProtocol::account_id(),
			CurrencyId::AUSD,
			u128_from_fixed_128(balance),
		));
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

	pub fn pool_liquidity(mut self, pool: LiquidityPoolId, liquidity: Balance) -> Self {
		self.pool_liquidities.push((pool, liquidity));
		self.endowed_accounts
			.push((MOCK_LIQUIDITY_LOCK_ACCOUNT, CurrencyId::AUSD, liquidity));
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
			margin_protocol_threshold: vec![
				(
					EUR_USD_PAIR,
					RiskThreshold::default(),
					RiskThreshold::default(),
					RiskThreshold::default(),
				),
				(
					JPY_EUR_PAIR,
					RiskThreshold::default(),
					RiskThreshold::default(),
					RiskThreshold::default(),
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
