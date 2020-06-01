#![cfg(feature = "std")]
// #[cfg(test)] doesn't work for some reason

use super::*;

use crate::{
	AccountId,
	CurrencyId::{self, AUSD, FEUR},
	LiquidityPoolId, MinimumCount, Moment, Runtime, DOLLARS,
};
use frame_support::{assert_ok, parameter_types, traits::OnFinalize, traits::OnInitialize};

use margin_liquidity_pools::SwapRate;
use margin_protocol::RiskThreshold;
use margin_protocol_rpc_runtime_api::runtime_decl_for_MarginProtocolApi::MarginProtocolApi;
use module_primitives::{Balance, IdentityInfo, Leverage, Leverages, TradingPair};
use module_traits::Treasury;
use orml_prices::Price;
use orml_traits::{BasicCurrency, MultiCurrency, PriceProvider};
use pallet_indices::address::Address;
use sp_arithmetic::Fixed128;
use sp_runtime::{DispatchResult, Permill};
use std::ops::Range;
use synthetic_protocol_rpc_runtime_api::runtime_decl_for_SyntheticProtocolApi::SyntheticProtocolApi;

pub type PositionId = u64;
pub type ModuleSyntheticProtocol = synthetic_protocol::Module<Runtime>;
pub type ModuleMarginProtocol = margin_protocol::Module<Runtime>;
pub type ModuleTokens = synthetic_tokens::Module<Runtime>;
pub type ModuleOracle = orml_oracle::Module<Runtime>;
pub type ModulePrices = orml_prices::DefaultPriceProvider<CurrencyId, ModuleOracle>;
pub type MarginLiquidityPools = margin_liquidity_pools::Module<Runtime>;
pub type SyntheticLiquidityPools = synthetic_liquidity_pools::Module<Runtime>;
pub type Timestamp = pallet_timestamp::Module<Runtime>;

pub const LIQUIDITY_POOL_ID_0: LiquidityPoolId = 0;
pub const ONE_MINUTE: u64 = 60;

pub const EUR_USD: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};

pub const JPY_USD: TradingPair = TradingPair {
	base: CurrencyId::FJPY,
	quote: CurrencyId::AUSD,
};

pub const JPY_EUR: TradingPair = TradingPair {
	base: CurrencyId::FJPY,
	quote: CurrencyId::FEUR,
};

pub fn risk_threshold(margin_call_percent: u32, stop_out_percent: u32) -> RiskThreshold {
	RiskThreshold {
		margin_call: Permill::from_percent(margin_call_percent),
		stop_out: Permill::from_percent(stop_out_percent),
	}
}

parameter_types! {
	pub const POOL: AccountId = AccountId::from([0u8; 32]);
	pub const ALICE: AccountId = AccountId::from([1u8; 32]);
	pub const BOB: AccountId = AccountId::from([2u8; 32]);

	pub const OracleList: Vec<AccountId> = vec![
		AccountId::from([100u8; 32]),
		AccountId::from([101u8; 32]),
		AccountId::from([102u8; 32]),
		AccountId::from([103u8; 32]),
		AccountId::from([104u8; 32]),
		AccountId::from([105u8; 32]),
		AccountId::from([106u8; 32]),
		AccountId::from([107u8; 32]),
		AccountId::from([108u8; 32]),
		AccountId::from([109u8; 32]),
	];
}

pub fn origin_of(who: &AccountId) -> <Runtime as system::Trait>::Origin {
	<Runtime as system::Trait>::Origin::signed((*who).clone())
}

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
		let mut t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![(POOL::get(), 100_000 * DOLLARS)],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_membership::GenesisConfig::<Runtime, pallet_membership::Instance3> {
			members: OracleList::get(),
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		margin_protocol::GenesisConfig {
			risk_thresholds: vec![
				(
					EUR_USD,
					risk_threshold(3, 1),
					risk_threshold(30, 10),
					risk_threshold(30, 10),
				),
				(
					JPY_USD,
					risk_threshold(3, 1),
					risk_threshold(30, 10),
					risk_threshold(30, 10),
				),
				(
					JPY_EUR,
					risk_threshold(3, 1),
					risk_threshold(30, 10),
					risk_threshold(30, 10),
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

pub fn set_oracle_price(prices: Vec<(CurrencyId, Price)>) -> DispatchResult {
	ModuleOracle::on_finalize(0);
	for i in 1..=MinimumCount::get() {
		assert_ok!(ModuleOracle::feed_values(
			<Runtime as system::Trait>::Origin::NONE,
			// origin_of(&OracleList::get()[i as usize]),
			prices.clone(),
			i as u32,
			Default::default()
		));
	}
	get_price();
	Ok(())
}

pub fn get_price() {
	ModulePrices::get_price(FEUR, AUSD);
}

pub fn dollar(amount: u128) -> u128 {
	amount.saturating_mul(Price::accuracy())
}

pub fn cent(amount: u128) -> u128 {
	amount.saturating_mul(Price::accuracy()) / 100
}

pub fn fixed_128_dollar(amount: i128) -> Fixed128 {
	Fixed128::saturating_from_integer(amount)
}

pub fn one_percent() -> Fixed128 {
	Fixed128::reciprocal(Fixed128::saturating_from_integer(100)).unwrap()
}

pub fn negative_one_percent() -> Fixed128 {
	Fixed128::reciprocal(Fixed128::saturating_from_integer(-100)).unwrap()
}

pub fn multi_currency_balance(who: &AccountId, currency_id: CurrencyId) -> Balance {
	<Runtime as synthetic_protocol::Trait>::MultiCurrency::free_balance(currency_id, &who)
}

pub fn native_currency_balance(who: &AccountId) -> Balance {
	Balances::free_balance(who)
}

pub fn synthetic_create_pool() -> DispatchResult {
	BaseLiquidityPoolsForSynthetic::create_pool(origin_of(&POOL::get()))?;
	BaseLiquidityPoolsForSynthetic::create_pool(origin_of(&POOL::get()))
}

pub fn synthetic_disable_pool(who: &AccountId) -> DispatchResult {
	BaseLiquidityPoolsForSynthetic::disable_pool(origin_of(who), LIQUIDITY_POOL_ID_0)
}

pub fn synthetic_remove_pool(who: &AccountId) -> DispatchResult {
	BaseLiquidityPoolsForSynthetic::remove_pool(origin_of(who), LIQUIDITY_POOL_ID_0)
}

pub fn synthetic_set_identity() -> DispatchResult {
	let identity = IdentityInfo {
		legal: "laminar".as_bytes().to_vec(),
		display: vec![],
		web: vec![],
		email: vec![],
		image_url: vec![],
	};

	BaseLiquidityPoolsForSynthetic::set_identity(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, identity)
}

pub fn synthetic_verify_identity() -> DispatchResult {
	BaseLiquidityPoolsForSynthetic::verify_identity(<Runtime as system::Trait>::Origin::ROOT, LIQUIDITY_POOL_ID_0)
}

pub fn synthetic_clear_identity() -> DispatchResult {
	BaseLiquidityPoolsForSynthetic::clear_identity(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0)
}

pub fn synthetic_transfer_liquidity_pool(who: &AccountId, pool_id: LiquidityPoolId, to: AccountId) -> DispatchResult {
	BaseLiquidityPoolsForSynthetic::transfer_liquidity_pool(origin_of(who), pool_id, to)
}

pub fn synthetic_set_enabled_trades() -> DispatchResult {
	SyntheticLiquidityPools::set_synthetic_enabled(
		origin_of(&POOL::get()),
		LIQUIDITY_POOL_ID_0,
		CurrencyId::FEUR,
		true,
	)?;
	SyntheticLiquidityPools::set_synthetic_enabled(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, CurrencyId::FJPY, true)
}

pub fn synthetic_deposit_liquidity(who: &AccountId, amount: Balance) -> DispatchResult {
	BaseLiquidityPoolsForSynthetic::deposit_liquidity(origin_of(who), LIQUIDITY_POOL_ID_0, amount)
}

pub fn synthetic_withdraw_liquidity(who: &AccountId, amount: Balance) -> DispatchResult {
	BaseLiquidityPoolsForSynthetic::withdraw_liquidity(origin_of(who), LIQUIDITY_POOL_ID_0, amount)
}

pub fn synthetic_buy(who: &AccountId, currency_id: CurrencyId, amount: Balance) -> DispatchResult {
	ModuleSyntheticProtocol::mint(
		origin_of(who),
		LIQUIDITY_POOL_ID_0,
		currency_id,
		amount,
		Price::from_rational(10, 1),
	)
}

pub fn synthetic_sell(who: &AccountId, currency_id: CurrencyId, amount: Balance) -> DispatchResult {
	ModuleSyntheticProtocol::redeem(
		origin_of(who),
		LIQUIDITY_POOL_ID_0,
		currency_id,
		amount,
		Price::from_rational(1, 10),
	)
}

// AUSD balance
pub fn collateral_balance(who: &AccountId) -> Balance {
	<Runtime as synthetic_protocol::Trait>::CollateralCurrency::free_balance(&who)
}

pub fn synthetic_balance() -> Balance {
	<Runtime as synthetic_protocol::Trait>::CollateralCurrency::free_balance(&ModuleTokens::account_id())
}

pub fn synthetic_set_min_additional_collateral_ratio(permill: Permill) -> DispatchResult {
	SyntheticLiquidityPools::set_min_additional_collateral_ratio(<Runtime as system::Trait>::Origin::ROOT, permill)
}

pub fn synthetic_set_additional_collateral_ratio(currency_id: CurrencyId, permill: Permill) -> DispatchResult {
	SyntheticLiquidityPools::set_additional_collateral_ratio(
		origin_of(&POOL::get()),
		LIQUIDITY_POOL_ID_0,
		currency_id,
		Some(permill),
	)
}

pub fn synthetic_set_spread(currency_id: CurrencyId, spread: Balance) -> DispatchResult {
	SyntheticLiquidityPools::set_spread(
		origin_of(&POOL::get()),
		LIQUIDITY_POOL_ID_0,
		currency_id,
		spread,
		spread,
	)
}

pub fn synthetic_liquidity() -> Balance {
	BaseLiquidityPoolsForSynthetic::balances(LIQUIDITY_POOL_ID_0)
}

pub fn synthetic_add_collateral(who: &AccountId, currency_id: CurrencyId, amount: Balance) -> DispatchResult {
	ModuleSyntheticProtocol::add_collateral(origin_of(who), LIQUIDITY_POOL_ID_0, currency_id, amount)
}

pub fn synthetic_liquidate(who: &AccountId, currency_id: CurrencyId, amount: Balance) -> DispatchResult {
	ModuleSyntheticProtocol::liquidate(origin_of(who), LIQUIDITY_POOL_ID_0, currency_id, amount)
}

pub fn synthetic_pool_info(currency_id: CurrencyId) -> Option<SyntheticProtocolPoolInfo> {
	<Runtime as SyntheticProtocolApi<Block, AccountId>>::pool_info(LIQUIDITY_POOL_ID_0, currency_id)
}

pub fn margin_create_pool() -> DispatchResult {
	BaseLiquidityPoolsForMargin::create_pool(origin_of(&POOL::get()))
}

pub fn margin_disable_pool(who: &AccountId) -> DispatchResult {
	BaseLiquidityPoolsForMargin::disable_pool(origin_of(who), LIQUIDITY_POOL_ID_0)
}

pub fn margin_remove_pool(who: &AccountId) -> DispatchResult {
	BaseLiquidityPoolsForMargin::remove_pool(origin_of(who), LIQUIDITY_POOL_ID_0)
}

pub fn margin_deposit_liquidity(who: &AccountId, amount: Balance) -> DispatchResult {
	BaseLiquidityPoolsForMargin::deposit_liquidity(origin_of(who), LIQUIDITY_POOL_ID_0, amount)
}

pub fn margin_set_enabled_trades() -> DispatchResult {
	MarginLiquidityPools::set_enabled_trades(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, EUR_USD, Leverages::all())?;
	MarginLiquidityPools::set_enabled_trades(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, JPY_EUR, Leverages::all())?;
	MarginLiquidityPools::set_enabled_trades(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, JPY_USD, Leverages::all())
}

pub fn margin_withdraw_liquidity(who: &AccountId, amount: Balance) -> DispatchResult {
	BaseLiquidityPoolsForMargin::withdraw_liquidity(origin_of(who), LIQUIDITY_POOL_ID_0, amount)
}

pub fn margin_set_spread(pair: TradingPair, spread: Balance) -> DispatchResult {
	MarginLiquidityPools::set_spread(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, pair, spread, spread)
}

pub fn margin_set_accumulate(pair: TradingPair, frequency: Moment, offset: Moment) -> DispatchResult {
	MarginLiquidityPools::set_accumulate(<Runtime as system::Trait>::Origin::ROOT, pair, frequency, offset)
}

pub fn margin_enable_trading_pair(pair: TradingPair) -> DispatchResult {
	MarginLiquidityPools::enable_trading_pair(<Runtime as system::Trait>::Origin::ROOT, pair)
}

pub fn margin_disable_trading_pair(pair: TradingPair) -> DispatchResult {
	MarginLiquidityPools::disable_trading_pair(<Runtime as system::Trait>::Origin::ROOT, pair)
}

pub fn margin_liquidity_pool_enable_trading_pair(pair: TradingPair) -> DispatchResult {
	MarginLiquidityPools::liquidity_pool_enable_trading_pair(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, pair)
}

pub fn margin_liquidity_pool_disable_trading_pair(pair: TradingPair) -> DispatchResult {
	MarginLiquidityPools::liquidity_pool_disable_trading_pair(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, pair)
}

pub fn margin_set_mock_swap_rate(pair: TradingPair) -> DispatchResult {
	let mock_swap_rate: SwapRate = SwapRate {
		long: Fixed128::reciprocal(Fixed128::saturating_from_integer(-100)).unwrap(),
		short: Fixed128::reciprocal(Fixed128::saturating_from_integer(100)).unwrap(),
	};

	MarginLiquidityPools::set_swap_rate(<Runtime as system::Trait>::Origin::ROOT, pair, mock_swap_rate)
}

pub fn margin_set_swap_rate(pair: TradingPair, long_rate: Fixed128, short_rate: Fixed128) -> DispatchResult {
	let swap_rate: SwapRate = SwapRate {
		long: long_rate,
		short: short_rate,
	};
	MarginLiquidityPools::set_swap_rate(<Runtime as system::Trait>::Origin::ROOT, pair, swap_rate)
}

pub fn margin_set_additional_swap(rate: Fixed128) -> DispatchResult {
	MarginLiquidityPools::set_additional_swap(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, rate)
}

pub fn margin_set_max_spread(pair: TradingPair, max_spread: Balance) -> DispatchResult {
	MarginLiquidityPools::set_max_spread(<Runtime as system::Trait>::Origin::ROOT, pair, max_spread)
}

pub fn margin_set_min_leveraged_amount(amount: Balance) -> DispatchResult {
	MarginLiquidityPools::set_min_leveraged_amount(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, amount)
}

pub fn margin_set_default_min_leveraged_amount(amount: Balance) -> DispatchResult {
	MarginLiquidityPools::set_default_min_leveraged_amount(<Runtime as system::Trait>::Origin::ROOT, amount)
}

pub fn margin_balance(who: &AccountId) -> Fixed128 {
	ModuleMarginProtocol::balances(who, LIQUIDITY_POOL_ID_0)
}

pub fn margin_liquidity() -> Balance {
	BaseLiquidityPoolsForMargin::balances(LIQUIDITY_POOL_ID_0)
}

pub fn margin_open_position(
	who: &AccountId,
	pair: TradingPair,
	leverage: Leverage,
	amount: Balance,
	price: Price,
) -> DispatchResult {
	ModuleMarginProtocol::open_position(origin_of(who), LIQUIDITY_POOL_ID_0, pair, leverage, amount, price)
}

pub fn margin_close_position(who: &AccountId, position_id: PositionId, price: Price) -> DispatchResult {
	ModuleMarginProtocol::close_position(origin_of(who), position_id, price)
}

pub fn margin_deposit(who: &AccountId, amount: Balance) -> DispatchResult {
	ModuleMarginProtocol::deposit(origin_of(who), LIQUIDITY_POOL_ID_0, amount)
}

pub fn margin_withdraw(who: &AccountId, amount: Balance) -> DispatchResult {
	ModuleMarginProtocol::withdraw(origin_of(who), LIQUIDITY_POOL_ID_0, amount)
}

pub fn margin_pool_required_deposit() -> Fixed128 {
	ModuleMarginProtocol::pool_required_deposit(LIQUIDITY_POOL_ID_0).unwrap()
}

pub fn margin_trader_margin_call(who: &AccountId) -> DispatchResult {
	ModuleMarginProtocol::trader_margin_call(
		<Runtime as system::Trait>::Origin::NONE,
		Address::from(who.clone()),
		LIQUIDITY_POOL_ID_0,
	)
}

pub fn margin_trader_become_safe(who: &AccountId) -> DispatchResult {
	ModuleMarginProtocol::trader_become_safe(
		<Runtime as system::Trait>::Origin::NONE,
		Address::from(who.clone()),
		LIQUIDITY_POOL_ID_0,
	)
}

pub fn margin_trader_stop_out(who: &AccountId) -> DispatchResult {
	ModuleMarginProtocol::trader_stop_out(
		<Runtime as system::Trait>::Origin::NONE,
		Address::from(who.clone()),
		LIQUIDITY_POOL_ID_0,
	)
}

pub fn margin_liquidity_pool_margin_call() -> DispatchResult {
	ModuleMarginProtocol::liquidity_pool_margin_call(<Runtime as system::Trait>::Origin::NONE, LIQUIDITY_POOL_ID_0)
}

pub fn margin_liquidity_pool_become_safe() -> DispatchResult {
	ModuleMarginProtocol::liquidity_pool_become_safe(<Runtime as system::Trait>::Origin::NONE, LIQUIDITY_POOL_ID_0)
}

pub fn margin_liquidity_pool_force_close() -> DispatchResult {
	ModuleMarginProtocol::liquidity_pool_force_close(<Runtime as system::Trait>::Origin::NONE, LIQUIDITY_POOL_ID_0)
}

pub fn margin_held(who: &AccountId) -> Fixed128 {
	ModuleMarginProtocol::margin_held(who, LIQUIDITY_POOL_ID_0)
}

pub fn free_margin(who: &AccountId) -> Fixed128 {
	ModuleMarginProtocol::free_margin(who, LIQUIDITY_POOL_ID_0).unwrap()
}

pub fn margin_equity(who: &AccountId) -> Fixed128 {
	ModuleMarginProtocol::equity_of_trader(who, LIQUIDITY_POOL_ID_0).unwrap()
}

pub fn margin_execute_time(range: Range<Moment>) {
	for i in range {
		System::set_block_number(i as u32);
		Timestamp::set_timestamp(i * 1000);
		MarginLiquidityPools::on_initialize(i as u32);

		//use module_traits::MarginProtocolLiquidityPools;
		//println!(
		//	"execute_block {:?}, accumulated_long_rate = {:?}, accumulated_short_rate = {:?}",
		//	i,
		//	MarginLiquidityPools::get_accumulated_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, true),
		//	MarginLiquidityPools::get_accumulated_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, false)
		//);
	}
}

pub fn margin_set_risk_threshold(
	pair: TradingPair,
	trader: Option<RiskThreshold>,
	enp: Option<RiskThreshold>,
	ell: Option<RiskThreshold>,
) -> DispatchResult {
	ModuleMarginProtocol::set_trading_pair_risk_threshold(
		<Runtime as system::Trait>::Origin::ROOT,
		pair,
		trader,
		enp,
		ell,
	)
}

pub fn treasury_balance() -> Balance {
	let account_id = MockLaminarTreasury::account_id();
	<Runtime as synthetic_protocol::Trait>::CollateralCurrency::free_balance(&account_id)
}

pub fn margin_trader_info(who: &AccountId) -> TraderInfo {
	Runtime::trader_info(who.clone(), LIQUIDITY_POOL_ID_0)
}

pub fn margin_pool_info() -> Option<PoolInfo> {
	<Runtime as MarginProtocolApi<Block, AccountId>>::pool_info(LIQUIDITY_POOL_ID_0)
}

pub fn margin_set_identity() -> DispatchResult {
	let identity = IdentityInfo {
		legal: "laminar".as_bytes().to_vec(),
		display: vec![],
		web: vec![],
		email: vec![],
		image_url: vec![],
	};

	BaseLiquidityPoolsForMargin::set_identity(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, identity)
}

pub fn margin_verify_identity() -> DispatchResult {
	BaseLiquidityPoolsForMargin::verify_identity(<Runtime as system::Trait>::Origin::ROOT, LIQUIDITY_POOL_ID_0)
}

pub fn margin_clear_identity() -> DispatchResult {
	BaseLiquidityPoolsForMargin::clear_identity(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0)
}

pub fn margin_transfer_liquidity_pool(who: &AccountId, pool_id: LiquidityPoolId, to: AccountId) -> DispatchResult {
	BaseLiquidityPoolsForMargin::transfer_liquidity_pool(origin_of(who), pool_id, to)
}
