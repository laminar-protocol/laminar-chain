/// tests for this module

#[cfg(test)]

mod tests {
	use frame_support::{assert_noop, assert_ok, parameter_types};
	use laminar_runtime::{
		AccountId, BlockNumber,
		CurrencyId::{self, AUSD, FEUR, FJPY},
		LiquidityPoolId, MaxSwap, MinimumCount, MockLaminarTreasury, Runtime,
	};

	type PositionId = u64;
	use margin_protocol::RiskThreshold;
	use module_primitives::{
		Balance,
		Leverage::{self, *},
		Leverages, TradingPair,
	};
	use module_traits::{LiquidityPoolManager, MarginProtocolLiquidityPools, Treasury};
	use orml_prices::Price;
	use orml_traits::{BasicCurrency, MultiCurrency, PriceProvider};
	use orml_utilities::Fixed128;
	use pallet_indices::address::Address;
	use sp_runtime::{traits::OnFinalize, traits::OnInitialize, DispatchResult, Permill};

	fn origin_of(who: &AccountId) -> <Runtime as system::Trait>::Origin {
		<Runtime as system::Trait>::Origin::signed((*who).clone())
	}

	type ModuleMarginProtocol = margin_protocol::Module<Runtime>;
	type ModuleTokens = synthetic_tokens::Module<Runtime>;
	type ModuleOracle = orml_oracle::Module<Runtime>;
	type ModulePrices = orml_prices::Module<Runtime>;
	type MarginLiquidityPools = margin_liquidity_pools::Module<Runtime>;

	const LIQUIDITY_POOL_ID_0: LiquidityPoolId = 0;
	const LIQUIDITY_POOL_ID_1: LiquidityPoolId = 1;

	const EUR_USD: TradingPair = TradingPair {
		base: CurrencyId::AUSD,
		quote: CurrencyId::FEUR,
	};

	const JPY_EUR: TradingPair = TradingPair {
		base: CurrencyId::FEUR,
		quote: CurrencyId::FJPY,
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

			margin_protocol::GenesisConfig {
				trader_risk_threshold: RiskThreshold {
					margin_call: Permill::from_percent(3),
					stop_out: Permill::from_percent(1),
				},
				liquidity_pool_enp_threshold: RiskThreshold {
					margin_call: Permill::from_percent(30),
					stop_out: Permill::from_percent(10),
				},
				liquidity_pool_ell_threshold: RiskThreshold {
					margin_call: Permill::from_percent(30),
					stop_out: Permill::from_percent(10),
				},
			}
			.assimilate_storage(&mut t)
			.unwrap();

			t.into()
		}
	}

	fn set_enabled_trades() -> DispatchResult {
		MarginLiquidityPools::set_enabled_trades(
			origin_of(&POOL::get()),
			LIQUIDITY_POOL_ID_0,
			EUR_USD,
			Leverages::all(),
		)?;
		MarginLiquidityPools::set_enabled_trades(
			origin_of(&POOL::get()),
			LIQUIDITY_POOL_ID_0,
			JPY_EUR,
			Leverages::all(),
		)
	}

	fn set_oracle_price(prices: Vec<(CurrencyId, Price)>) -> DispatchResult {
		ModuleOracle::on_finalize(0);
		for i in 1..=MinimumCount::get() {
			assert_ok!(ModuleOracle::feed_values(
				origin_of(&OracleList::get()[i as usize]),
				prices.clone()
			));
		}
		Ok(())
	}

	fn get_price() {
		ModulePrices::get_price(AUSD, FEUR);
	}

	fn dollar(amount: u128) -> u128 {
		amount.saturating_mul(Price::accuracy())
	}

	fn one_percent() -> Fixed128 {
		Fixed128::recip(&Fixed128::from_natural(100)).unwrap()
	}

	fn create_pool() -> DispatchResult {
		MarginLiquidityPools::create_pool(origin_of(&POOL::get()))?;
		MarginLiquidityPools::create_pool(origin_of(&POOL::get()))
	}

	fn multi_currency_balance(who: &AccountId, currency_id: CurrencyId) -> Balance {
		<Runtime as synthetic_protocol::Trait>::MultiCurrency::free_balance(currency_id, &who)
	}

	// AUSD balance
	fn collateral_balance(who: &AccountId) -> Balance {
		<Runtime as synthetic_protocol::Trait>::CollateralCurrency::free_balance(&who)
	}

	fn margin_disable_pool(who: &AccountId) -> DispatchResult {
		MarginLiquidityPools::disable_pool(origin_of(who), LIQUIDITY_POOL_ID_0)
	}

	fn margin_remove_pool(who: &AccountId) -> DispatchResult {
		MarginLiquidityPools::remove_pool(origin_of(who), LIQUIDITY_POOL_ID_0)
	}

	fn margin_deposit_liquidity(who: &AccountId, amount: Balance) -> DispatchResult {
		MarginLiquidityPools::deposit_liquidity(origin_of(who), LIQUIDITY_POOL_ID_0, amount)
	}

	fn margin_withdraw_liquidity(who: &AccountId, amount: Balance) -> DispatchResult {
		MarginLiquidityPools::withdraw_liquidity(origin_of(who), LIQUIDITY_POOL_ID_0, amount)
	}

	fn margin_set_spread(pair: TradingPair, spread: Permill) -> DispatchResult {
		MarginLiquidityPools::set_spread(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, pair, spread, spread)
	}

	fn margin_set_accumulate(pair: TradingPair, frequency: BlockNumber, offset: BlockNumber) -> DispatchResult {
		MarginLiquidityPools::set_accumulate(<Runtime as system::Trait>::Origin::ROOT, pair, frequency, offset)
	}

	fn margin_enable_trading_pair(pair: TradingPair) -> DispatchResult {
		MarginLiquidityPools::enable_trading_pair(<Runtime as system::Trait>::Origin::ROOT, pair)
	}

	fn margin_disable_trading_pair(pair: TradingPair) -> DispatchResult {
		MarginLiquidityPools::disable_trading_pair(<Runtime as system::Trait>::Origin::ROOT, pair)
	}

	fn margin_liquidity_pool_enable_trading_pair(pair: TradingPair) -> DispatchResult {
		MarginLiquidityPools::liquidity_pool_enable_trading_pair(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, pair)
	}

	fn margin_liquidity_pool_disable_trading_pair(pair: TradingPair) -> DispatchResult {
		MarginLiquidityPools::liquidity_pool_disable_trading_pair(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, pair)
	}

	fn margin_update_swap(pair: TradingPair, rate: Fixed128) -> DispatchResult {
		MarginLiquidityPools::update_swap(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, pair, rate)
	}

	fn margin_set_max_spread(pair: TradingPair, max_spread: Permill) -> DispatchResult {
		MarginLiquidityPools::set_max_spread(<Runtime as system::Trait>::Origin::ROOT, pair, max_spread)
	}

	fn margin_set_min_leveraged_amount(amount: Balance) -> DispatchResult {
		MarginLiquidityPools::set_min_leveraged_amount(origin_of(&POOL::get()), LIQUIDITY_POOL_ID_0, amount)
	}

	fn margin_set_default_min_leveraged_amount(amount: Balance) -> DispatchResult {
		MarginLiquidityPools::set_default_min_leveraged_amount(<Runtime as system::Trait>::Origin::ROOT, amount)
	}

	fn margin_balance(who: &AccountId) -> Balance {
		ModuleMarginProtocol::balances(who)
	}

	fn margin_liquidity() -> Balance {
		MarginLiquidityPools::balances(LIQUIDITY_POOL_ID_0)
	}

	fn margin_open_position(
		who: &AccountId,
		pair: TradingPair,
		leverage: Leverage,
		amount: Balance,
		price: Price,
	) -> DispatchResult {
		ModuleMarginProtocol::open_position(origin_of(who), LIQUIDITY_POOL_ID_0, pair, leverage, amount, price)
	}

	fn margin_close_position(who: &AccountId, position_id: PositionId, price: Price) -> DispatchResult {
		ModuleMarginProtocol::close_position(origin_of(who), position_id, price)
	}

	fn margin_deposit(who: &AccountId, amount: Balance) -> DispatchResult {
		ModuleMarginProtocol::deposit(origin_of(who), amount)
	}

	fn margin_withdraw(who: &AccountId, amount: Balance) -> DispatchResult {
		ModuleMarginProtocol::withdraw(origin_of(who), amount)
	}

	fn margin_get_required_deposit() -> Balance {
		ModuleMarginProtocol::get_required_deposit(LIQUIDITY_POOL_ID_0).unwrap()
	}

	fn margin_trader_margin_call(who: &AccountId) -> DispatchResult {
		ModuleMarginProtocol::trader_margin_call(<Runtime as system::Trait>::Origin::NONE, Address::from(who.clone()))
	}

	fn margin_trader_become_safe(who: &AccountId) -> DispatchResult {
		ModuleMarginProtocol::trader_become_safe(<Runtime as system::Trait>::Origin::NONE, Address::from(who.clone()))
	}

	fn margin_trader_liquidate(who: &AccountId) -> DispatchResult {
		ModuleMarginProtocol::trader_liquidate(<Runtime as system::Trait>::Origin::NONE, Address::from(who.clone()))
	}

	fn margin_liquidity_pool_margin_call() -> DispatchResult {
		ModuleMarginProtocol::liquidity_pool_margin_call(<Runtime as system::Trait>::Origin::NONE, LIQUIDITY_POOL_ID_0)
	}

	fn margin_liquidity_pool_become_safe() -> DispatchResult {
		ModuleMarginProtocol::liquidity_pool_become_safe(<Runtime as system::Trait>::Origin::NONE, LIQUIDITY_POOL_ID_0)
	}

	fn margin_liquidity_pool_liquidate() -> DispatchResult {
		ModuleMarginProtocol::liquidity_pool_liquidate(<Runtime as system::Trait>::Origin::NONE, LIQUIDITY_POOL_ID_0)
	}

	#[test]
	fn test_margin_liquidity_pools() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit_liquidity(&ALICE::get(), dollar(5000)));
				assert_noop!(
					margin_deposit_liquidity(&ALICE::get(), dollar(6_000)),
					orml_tokens::Error::<Runtime>::BalanceTooLow
				);
				assert_eq!(margin_liquidity(), dollar(15000));

				assert_noop!(
					margin_withdraw_liquidity(&ALICE::get(), dollar(5000)),
					margin_liquidity_pools::Error::<Runtime>::NoPermission
				);

				assert_eq!(margin_get_required_deposit(), 0);

				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));
				assert_ok!(margin_set_max_spread(EUR_USD, Permill::from_percent(2)));
				assert_noop!(
					margin_set_spread(EUR_USD, Permill::from_percent(3)),
					margin_liquidity_pools::Error::<Runtime>::SpreadTooHigh
				);
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(set_enabled_trades());

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));
				assert_noop!(
					margin_update_swap(EUR_USD, MaxSwap::get().checked_add(&one_percent()).unwrap()),
					margin_liquidity_pools::Error::<Runtime>::SwapRateTooHigh
				);

				assert_noop!(
					margin_liquidity_pool_enable_trading_pair(EUR_USD),
					margin_liquidity_pools::Error::<Runtime>::TradingPairNotEnabled
				);

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));
				assert_ok!(margin_disable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_disable_trading_pair(EUR_USD));
				assert_ok!(margin_withdraw_liquidity(&POOL::get(), dollar(10_000)));
				assert_eq!(margin_liquidity(), dollar(5000));
				assert_ok!(margin_disable_pool(&POOL::get()));
				assert_ok!(margin_remove_pool(&POOL::get()));
				assert_eq!(collateral_balance(&POOL::get()), dollar(15000));
			});
	}

	#[test]
	fn test_margin_open_and_close() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));

				assert_ok!(margin_close_position(&ALICE::get(), 0, Price::from_rational(2, 1)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 3 * (1 - 0.01) = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				assert_eq!(margin_balance(&ALICE::get()), dollar(4700));
				assert_eq!(margin_liquidity(), dollar(10_300));
				assert_ok!(margin_withdraw(&ALICE::get(), dollar(4700)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(9700));
			});
	}

	#[test]
	fn test_margin_take_profit() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(4, 1))]));

				assert_ok!(margin_close_position(&ALICE::get(), 0, Price::from_rational(2, 1)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 4 * (1 - 0.01) = 3.96
				// profit = leveraged_held * (close_price - open_price)
				// 4650 = 5000 * (3.96 - 3.03)
				assert_eq!(margin_balance(&ALICE::get()), dollar(9650));
				assert_eq!(margin_liquidity(), dollar(5350));
			});
	}

	#[test]
	fn test_margin_stop_lost() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(28, 10))]));

				assert_ok!(margin_close_position(&ALICE::get(), 0, Price::from_rational(1, 1)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 2.8 * (1 - 0.01) = 2.772
				// profit = leveraged_held * (close_price - open_price)
				// -1290 = 5000 * (2.772 - 3.03)
				assert_eq!(margin_balance(&ALICE::get()), dollar(3710));
				assert_eq!(margin_liquidity(), dollar(11_290));
			});
	}

	#[test]
	fn test_margin_trader_liquidate() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				// margin = leveraged_amount * price / leverage
				// 1505 = 5000 * 3.01 / 10
				// 2.12409 = 3 * (1 - 1505 * 97% / 5000)
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(22, 10))]));
				get_price();
				assert_noop!(
					margin_trader_margin_call(&ALICE::get()),
					margin_protocol::Error::<Runtime>::SafeTrader
				);
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(21, 10))]));
				assert_ok!(margin_trader_margin_call(&ALICE::get()));
				assert_noop!(
					margin_trader_liquidate(&ALICE::get()),
					margin_protocol::Error::<Runtime>::NotReachedRiskThreshold
				);

				assert_noop!(
					margin_trader_become_safe(&ALICE::get()),
					margin_protocol::Error::<Runtime>::UnsafeTrader
				);
				// Price up become safe
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(22, 10))]));
				assert_ok!(margin_trader_become_safe(&ALICE::get()));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(21, 10))]));
				assert_ok!(margin_trader_margin_call(&ALICE::get()));

				// Deposit become safe
				assert_ok!(margin_deposit(&ALICE::get(), dollar(500)));
				assert_ok!(margin_trader_become_safe(&ALICE::get()));

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(19, 10))]));
				assert_ok!(margin_trader_liquidate(&ALICE::get()));

				assert_eq!(collateral_balance(&ALICE::get()), dollar(4500));
				assert_eq!(margin_balance(&ALICE::get()), 0);
				assert_eq!(margin_liquidity(), dollar(15_500));
			});
	}

	#[test]
	fn test_margin_liquidity_pool_liquidate() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(20_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				// 4.4 = 3 * (1 + 10000 * 70% / 3.01 / 5000)
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(41, 10))]));
				get_price();
				assert_noop!(
					margin_liquidity_pool_margin_call(),
					margin_protocol::Error::<Runtime>::SafePool
				);
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(42, 10))]));
				assert_ok!(margin_liquidity_pool_margin_call());
				assert_noop!(
					margin_liquidity_pool_liquidate(),
					margin_protocol::Error::<Runtime>::NotReachedRiskThreshold
				);

				assert_noop!(
					margin_liquidity_pool_become_safe(),
					margin_protocol::Error::<Runtime>::UnsafePool
				);
				// Price up become safe
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(41, 10))]));
				assert_ok!(margin_liquidity_pool_become_safe());
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(42, 10))]));
				assert_ok!(margin_liquidity_pool_margin_call());

				// Deposit become safe
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(500)));
				assert_ok!(margin_liquidity_pool_become_safe());

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(50, 10))]));
				assert_eq!(collateral_balance(&MockLaminarTreasury::account_id()), 0);
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_ok!(margin_liquidity_pool_liquidate());

				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 5 * (1 - 0.01) = 4.95
				// profit = leveraged_held * (close_price - open_price)
				// 9600 = 5000 * (4.95 - 3.03)
				assert_eq!(margin_balance(&ALICE::get()), dollar(14600));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_liquidity(), dollar(400));
				// penalty = leveraged_held * price * spread * 2
				// 500 = 5000 * 5 * 0.01 * 2
				assert_eq!(collateral_balance(&MockLaminarTreasury::account_id()), dollar(500));
			});
	}

	#[test]
	fn test_margin_multiple_users() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
				(BOB::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(20_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(9000)));
				assert_ok!(margin_deposit(&BOB::get(), dollar(9000)));
				assert_eq!(margin_liquidity(), dollar(20_000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(9000));
				assert_eq!(margin_balance(&BOB::get()), dollar(9000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(1000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(1000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				// ALICE open position
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::from_rational(4, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), dollar(9000));

				// BOB open position
				assert_ok!(margin_open_position(
					&BOB::get(),
					EUR_USD,
					ShortTen,
					dollar(6000),
					Price::from_rational(2, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), dollar(9000));

				// ALICE open position and BOB close position
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(31, 10))]));
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTwenty,
					dollar(1000),
					Price::from_rational(4, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), dollar(9000));
				assert_ok!(margin_close_position(&BOB::get(), 1, Price::from_rational(4, 1)));
				// open_price = 3 * (1 - 0.01) = 2.97
				// close_price = 3.1 * (1 + 0.01) = 3.131
				// profit = leveraged_held * (close_price - open_price)
				// 966 = 6000 * (3.131 - 2.97)
				assert_eq!(margin_balance(&BOB::get()), dollar(8034));

				// ALICE close position and BOB open position
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(29, 10))]));
				assert_ok!(margin_close_position(&ALICE::get(), 0, Price::from_rational(2, 1)));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 2.9 * (1 - 0.01) = 2.871
				// profit = leveraged_held * (close_price - open_price)
				// -795 = 5000 * (2.871 - 3.03)
				assert_eq!(margin_balance(&ALICE::get()), dollar(8205));
				assert_ok!(margin_open_position(
					&BOB::get(),
					EUR_USD,
					ShortTwenty,
					dollar(2000),
					Price::from_rational(2, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), dollar(8034));

				// close all
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(28, 10))]));
				assert_ok!(margin_close_position(&ALICE::get(), 2, Price::from_rational(2, 1)));
				// open_price = 3.1 * (1 + 0.01) = 3.131
				// close_price = 2.8 * (1 - 0.01) = 2.772
				// profit = leveraged_held * (close_price - open_price)
				// -359 = 1000 * (2.772 - 3.131)
				assert_eq!(margin_balance(&ALICE::get()), dollar(7846));
				assert_ok!(margin_close_position(&BOB::get(), 3, Price::from_rational(4, 1)));
				// open_price = 2.9 * (1 - 0.01) = 2.871
				// close_price = 2.8 * (1 + 0.01) = 2.828
				// profit = leveraged_held * (close_price - open_price)
				// -86 = 2000 * (2.828 - 2.871)
				assert_eq!(margin_balance(&BOB::get()), dollar(8120));
				assert_eq!(margin_liquidity(), dollar(22_034));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(1000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(1000));
			});
	}

	#[test]
	fn test_margin_multiple_users_multiple_currencies() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
				(BOB::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(20_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(9000)));
				assert_ok!(margin_deposit(&BOB::get(), dollar(9000)));
				assert_eq!(margin_liquidity(), dollar(20_000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(9000));
				assert_eq!(margin_balance(&BOB::get()), dollar(9000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(1000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(1000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1)),
					(FJPY, Price::from_rational(5, 1))
				]));
				assert_ok!(set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));
				assert_ok!(margin_set_spread(JPY_EUR, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_accumulate(JPY_EUR, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));
				assert_ok!(margin_update_swap(JPY_EUR, one_percent()));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_enable_trading_pair(JPY_EUR));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(JPY_EUR));

				// ALICE open position
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::from_rational(4, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), dollar(9000));

				// BOB open position
				assert_ok!(margin_open_position(
					&BOB::get(),
					JPY_EUR,
					ShortTen,
					dollar(6000),
					Price::from_rational(1, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), dollar(9000));

				// ALICE open position and BOB close position
				assert_ok!(set_oracle_price(vec![
					(FEUR, Price::from_rational(31, 10)),
					(FJPY, Price::from_rational(49, 10))
				]));
				assert_ok!(margin_open_position(
					&ALICE::get(),
					JPY_EUR,
					LongTwenty,
					dollar(1000),
					Price::from_rational(4, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), dollar(9000));
				assert_ok!(margin_close_position(&BOB::get(), 1, Price::from_rational(4, 1)));
				// open_price = 5/3 * (1 - 0.01) = 1.65
				// close_price = 4.9/3.1 * (1 + 0.01) = 1.596451612903226
				// profit = leveraged_held * (close_price - open_price)
				// -995.9999999999964 = 6000 * (1.596451612903226 - 1.65) * 3.1
				assert_eq!(margin_balance(&BOB::get()), 9996000000000000008400);

				// ALICE close position and BOB open position
				assert_ok!(set_oracle_price(vec![
					(FEUR, Price::from_rational(29, 10)),
					(FJPY, Price::from_rational(51, 10))
				]));
				assert_ok!(margin_close_position(&ALICE::get(), 0, Price::from_rational(2, 1)));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 2.9 * (1 - 0.01) = 2.871
				// profit = leveraged_held * (close_price - open_price)
				// -795 = 5000 * (2.871 - 3.03)
				assert_eq!(margin_balance(&ALICE::get()), dollar(8205));
				assert_ok!(margin_open_position(
					&BOB::get(),
					EUR_USD,
					ShortTwenty,
					dollar(2000),
					Price::from_rational(2, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), 9996000000000000008400);

				// close all
				assert_ok!(set_oracle_price(vec![
					(FEUR, Price::from_rational(28, 10)),
					(FJPY, Price::from_rational(52, 10))
				]));
				assert_ok!(margin_close_position(&ALICE::get(), 2, Price::from_rational(1, 1)));
				// open_price = 4.9/3.1 * (1 + 0.01) = 1.596451612903226
				// close_price = 5.2/2.8 * (1 - 0.01) = 1.838571428571429
				// profit = leveraged_held * (close_price - open_price)
				// 677.9354838709672 = 1000 * (1.838571428571429 - 1.596451612903226) * 2.8
				assert_eq!(margin_balance(&ALICE::get()), 8882935483870967742000);
				assert_ok!(margin_close_position(&BOB::get(), 3, Price::from_rational(4, 1)));
				// open_price = 2.9 * (1 - 0.01) = 2.871
				// close_price = 2.8 * (1 + 0.01) = 2.828
				// profit = leveraged_held * (close_price - open_price)
				// -86 = 2000 * (2.828 - 2.871)
				assert_eq!(margin_balance(&BOB::get()), 10082000000000000008400);
				assert_eq!(margin_liquidity(), 19035064516129032249600);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(1000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(1000));
			});
	}

	//TODO: should set long and short swaps for each trading pair.
	//#[test]
	fn test_margin_accumulate_swap() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_update_swap(EUR_USD, one_percent()));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				// LongTen
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));

				for i in 1..9 {
					MarginLiquidityPools::on_initialize(i);
				}

				assert_ok!(margin_close_position(&ALICE::get(), 0, Price::from_rational(2, 1)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 3 * (1 - 0.01) = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				// penalty = leveraged_held * (accumulated_swap_rate - open_accumulated_swap_rate)
				// 50 = 5000 * (0.01 - 0)
				assert_eq!(margin_balance(&ALICE::get()), dollar(4650));
				assert_eq!(margin_liquidity(), dollar(10350));

				// ShortTen
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					ShortTen,
					dollar(5000),
					Price::from_rational(2, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(4650));

				for i in 1..12 {
					MarginLiquidityPools::on_initialize(i);
					println!(
						"accumulated_swap_rate = {:?}",
						MarginLiquidityPools::get_accumulated_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD)
					);
				}

				assert_ok!(margin_close_position(&ALICE::get(), 1, Price::from_rational(4, 1)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 3 * (1 - 0.01) = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				// penalty = leveraged_held * (accumulated_swap_rate - open_accumulated_swap_rate)
				// 101.505 = 5000 * (0.030301 - 0.01)
				assert_eq!(margin_balance(&ALICE::get()), 4451505000000000000000);
				assert_eq!(margin_liquidity(), 10548495000000000000000);
				assert_ok!(margin_withdraw(&ALICE::get(), 4451505000000000000000));
				assert_eq!(collateral_balance(&ALICE::get()), 9451505000000000000000);
			});
	}
}
