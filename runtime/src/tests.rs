#![cfg(feature = "std")]
// #[cfg(test)] doesn't work for some reason

use super::*;

use crate::{
	AccountId, BlockNumber,
	CurrencyId::{self, AUSD, FEUR},
	LiquidityPoolId, MinimumCount, Runtime,
};
use frame_support::{assert_ok, parameter_types, traits::OnFinalize};

use margin_liquidity_pools::SwapRate;
use module_primitives::{AccumulateConfig, Balance, Leverage, Leverages, RiskThreshold, TradingPair};
use module_traits::LiquidityPoolManager;
use orml_prices::Price;
use orml_traits::{BasicCurrency, MultiCurrency, PriceProvider};
use orml_utilities::Fixed128;
use pallet_indices::address::Address;
use sp_runtime::{DispatchResult, Permill};
use sp_std::num::NonZeroI128;

pub type PositionId = u64;
pub type ModuleSyntheticProtocol = synthetic_protocol::Module<Runtime>;
pub type ModuleMarginProtocol = margin_protocol::Module<Runtime>;
pub type ModuleTokens = synthetic_tokens::Module<Runtime>;
pub type ModuleOracle = orml_oracle::Module<Runtime>;
pub type ModulePrices = orml_prices::DefaultPriceProvider<CurrencyId, ModuleOracle>;
pub type MarginLiquidityPools = margin_liquidity_pools::Module<Runtime>;
pub type SyntheticLiquidityPools = synthetic_liquidity_pools::Module<Runtime>;

pub const LIQUIDITY_POOL_ID_0: LiquidityPoolId = 0;

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

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_collective::GenesisConfig::<Runtime, pallet_collective::Instance3> {
			members: OracleList::get(),
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		margin_liquidity_pools::GenesisConfig::<Runtime> {
			default_min_leveraged_amount: 1000,
			margin_liquidity_config: vec![(
				// TradingPair
				TradingPair {
					base: CurrencyId::FEUR,
					quote: CurrencyId::AUSD,
				},
				// MaxSpread
				Permill::from_percent(1),
				// Accumulates
				AccumulateConfig {
					frequency: 10,
					offset: 1,
				},
				// SwapRates
				SwapRate {
					long: Fixed128::from_rational(1, NonZeroI128::new(100).unwrap()),
					short: Fixed128::from_rational(-1, NonZeroI128::new(100).unwrap()),
				},
				// trader_risk_threshold
				RiskThreshold {
					margin_call: Permill::from_percent(3),
					stop_out: Permill::from_percent(1),
				},
				// liquidity_pool_enp_threshold
				RiskThreshold {
					margin_call: Permill::from_percent(30),
					stop_out: Permill::from_percent(10),
				},
				// liquidity_pool_ell_threshold
				RiskThreshold {
					margin_call: Permill::from_percent(30),
					stop_out: Permill::from_percent(10),
				},
			)],
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
			origin_of(&OracleList::get()[i as usize]),
			prices.clone()
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

pub fn fixed_128_dollar(amount: i128) -> Fixed128 {
	Fixed128::from_natural(amount)
}

pub fn one_percent() -> Fixed128 {
	Fixed128::recip(&Fixed128::from_natural(100)).unwrap()
}

pub fn negative_one_percent() -> Fixed128 {
	Fixed128::recip(&Fixed128::from_natural(-100)).unwrap()
}

pub fn multi_currency_balance(who: &AccountId, currency_id: CurrencyId) -> Balance {
	<Runtime as synthetic_protocol::Trait>::MultiCurrency::free_balance(currency_id, &who)
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

pub fn synthetic_set_spread(currency_id: CurrencyId, spread: Permill) -> DispatchResult {
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

pub fn margin_set_spread(pair: TradingPair, spread: Permill) -> DispatchResult {
	MarginLiquidityPools::set_spread(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, pair, spread, spread)
}

pub fn margin_set_accumulate(pair: TradingPair, frequency: BlockNumber, offset: BlockNumber) -> DispatchResult {
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
		long: Fixed128::recip(&Fixed128::from_natural(-100)).unwrap(),
		short: Fixed128::recip(&Fixed128::from_natural(100)).unwrap(),
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

pub fn margin_set_max_spread(pair: TradingPair, max_spread: Permill) -> DispatchResult {
	MarginLiquidityPools::set_max_spread(<Runtime as system::Trait>::Origin::ROOT, pair, max_spread)
}

pub fn margin_set_min_leveraged_amount(amount: Balance) -> DispatchResult {
	MarginLiquidityPools::set_min_leveraged_amount(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, amount)
}

pub fn margin_set_default_min_leveraged_amount(amount: Balance) -> DispatchResult {
	MarginLiquidityPools::set_default_min_leveraged_amount(<Runtime as system::Trait>::Origin::ROOT, amount)
}

pub fn margin_balance(who: &AccountId) -> Fixed128 {
	ModuleMarginProtocol::balances(who)
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
	ModuleMarginProtocol::deposit(origin_of(who), amount)
}

pub fn margin_withdraw(who: &AccountId, amount: Balance) -> DispatchResult {
	ModuleMarginProtocol::withdraw(origin_of(who), amount)
}

pub fn margin_get_required_deposit() -> Balance {
	ModuleMarginProtocol::get_required_deposit(LIQUIDITY_POOL_ID_0).unwrap()
}

pub fn margin_trader_margin_call(who: &AccountId, pair: TradingPair) -> DispatchResult {
	ModuleMarginProtocol::trader_margin_call(
		<Runtime as system::Trait>::Origin::NONE,
		Address::from(who.clone()),
		pair,
	)
}

pub fn margin_trader_become_safe(who: &AccountId, pair: TradingPair) -> DispatchResult {
	ModuleMarginProtocol::trader_become_safe(
		<Runtime as system::Trait>::Origin::NONE,
		Address::from(who.clone()),
		pair,
	)
}

pub fn margin_trader_stop_out(who: &AccountId, pair: TradingPair) -> DispatchResult {
	ModuleMarginProtocol::trader_stop_out(
		<Runtime as system::Trait>::Origin::NONE,
		Address::from(who.clone()),
		pair,
	)
}

pub fn margin_liquidity_pool_margin_call(pair: TradingPair) -> DispatchResult {
	ModuleMarginProtocol::liquidity_pool_margin_call(
		<Runtime as system::Trait>::Origin::NONE,
		LIQUIDITY_POOL_ID_0,
		pair,
	)
}

pub fn margin_liquidity_pool_become_safe(pair: TradingPair) -> DispatchResult {
	ModuleMarginProtocol::liquidity_pool_become_safe(
		<Runtime as system::Trait>::Origin::NONE,
		LIQUIDITY_POOL_ID_0,
		pair,
	)
}

pub fn margin_liquidity_pool_force_close(pair: TradingPair) -> DispatchResult {
	ModuleMarginProtocol::liquidity_pool_force_close(
		<Runtime as system::Trait>::Origin::NONE,
		LIQUIDITY_POOL_ID_0,
		pair,
	)
}

pub fn margin_held(who: &AccountId) -> Fixed128 {
	ModuleMarginProtocol::margin_held(who)
}

pub fn free_margin(who: &AccountId) -> Fixed128 {
	ModuleMarginProtocol::free_margin(who).unwrap()
}

pub fn margin_equity(who: &AccountId) -> Fixed128 {
	ModuleMarginProtocol::equity_of_trader(who).unwrap()
}
