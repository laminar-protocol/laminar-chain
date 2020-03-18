/// tests for this module

#[cfg(test)]

mod tests {
	use frame_support::{assert_noop, assert_ok, parameter_types};
	pub use laminar_runtime::{
		AccountId,
		CurrencyId::{self, AUSD, FEUR, FJPY},
		LiquidityPoolId, MinimumCount, Runtime,
	};
	pub use module_primitives::{Balance, Leverage, Leverages, TradingPair};
	pub use orml_prices::Price;
	use orml_traits::{BasicCurrency, MultiCurrency};
	pub use sp_runtime::{traits::OnFinalize, traits::Zero, DispatchResult, Perbill, Permill};

	pub fn origin_of(who: &AccountId) -> <Runtime as system::Trait>::Origin {
		<Runtime as system::Trait>::Origin::signed((*who).clone())
	}

	pub type ModuleProtocol = synthetic_protocol::Module<Runtime>;
	pub type ModuleTokens = synthetic_tokens::Module<Runtime>;
	pub type ModuleOracle = orml_oracle::Module<Runtime>;
	pub type MarginLiquidityPools = margin_liquidity_pools::Module<Runtime>;
	pub type SyntheticLiquidityPools = synthetic_liquidity_pools::Module<Runtime>;

	const LIQUIDITY_POOL_ID: LiquidityPoolId = 0;

	const PAIR: TradingPair = TradingPair {
		base: CurrencyId::AUSD,
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

			t.into()
		}
	}

	pub fn create_pool() -> DispatchResult {
		MarginLiquidityPools::create_pool(origin_of(&POOL::get()))
			.and_then(|()| SyntheticLiquidityPools::create_pool(origin_of(&POOL::get())))
	}

	pub fn set_enabled_trades() -> DispatchResult {
		MarginLiquidityPools::set_enabled_trades(origin_of(&POOL::get()), LIQUIDITY_POOL_ID, PAIR, Leverages::all())
	}

	pub fn deposit_liquidity(amount: Balance) -> DispatchResult {
		SyntheticLiquidityPools::deposit_liquidity(origin_of(&POOL::get()), LIQUIDITY_POOL_ID, amount)
	}

	pub fn set_min_additional_collateral_ratio(permill: Permill) -> DispatchResult {
		SyntheticLiquidityPools::set_min_additional_collateral_ratio(<Runtime as system::Trait>::Origin::ROOT, permill)
	}

	pub fn set_additional_collateral_ratio(currency_id: CurrencyId, permill: Permill) -> DispatchResult {
		SyntheticLiquidityPools::set_additional_collateral_ratio(
			origin_of(&POOL::get()),
			LIQUIDITY_POOL_ID,
			currency_id,
			Some(permill),
		)
	}

	pub fn set_spread(currency_id: CurrencyId, permill: Permill) -> DispatchResult {
		MarginLiquidityPools::set_spread(origin_of(&POOL::get()), LIQUIDITY_POOL_ID, PAIR, permill, permill).and_then(
			|()| {
				SyntheticLiquidityPools::set_spread(
					origin_of(&POOL::get()),
					LIQUIDITY_POOL_ID,
					currency_id,
					permill,
					permill,
				)
			},
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

	fn buy(who: &AccountId, currency_id: CurrencyId, amount: Balance) -> DispatchResult {
		ModuleProtocol::mint(
			origin_of(who),
			LIQUIDITY_POOL_ID,
			currency_id,
			amount,
			Permill::from_percent(10),
		)
	}

	fn sell(who: &AccountId, currency_id: CurrencyId, amount: Balance) -> DispatchResult {
		ModuleProtocol::redeem(
			origin_of(who),
			LIQUIDITY_POOL_ID,
			currency_id,
			amount,
			Permill::from_percent(10),
		)
	}

	fn collateral_balance(who: &AccountId) -> Balance {
		<Runtime as synthetic_protocol::Trait>::CollateralCurrency::free_balance(&who)
	}

	fn synthetic_collateral_balance() -> Balance {
		<Runtime as synthetic_protocol::Trait>::CollateralCurrency::free_balance(&ModuleTokens::account_id())
	}

	fn synthetic_balance(who: &AccountId, currency_id: CurrencyId) -> Balance {
		<Runtime as synthetic_protocol::Trait>::MultiCurrency::free_balance(currency_id, &who)
	}

	fn synthetic_enabled(currency_id: CurrencyId, enabled: bool) -> DispatchResult {
		SyntheticLiquidityPools::set_synthetic_enabled(origin_of(&POOL::get()), LIQUIDITY_POOL_ID, currency_id, enabled)
	}

	fn liquidity() -> Balance {
		SyntheticLiquidityPools::balances(LIQUIDITY_POOL_ID)
	}

	fn add_collateral(who: &AccountId, currency_id: CurrencyId, amount: Balance) -> DispatchResult {
		ModuleProtocol::add_collateral(origin_of(who), LIQUIDITY_POOL_ID, currency_id, amount)
	}

	fn liquidate(who: &AccountId, currency_id: CurrencyId, amount: Balance) -> DispatchResult {
		ModuleProtocol::liquidate(origin_of(who), LIQUIDITY_POOL_ID, currency_id, amount)
	}

	fn remove_pool(who: &AccountId) -> DispatchResult {
		SyntheticLiquidityPools::remove_pool(origin_of(who), LIQUIDITY_POOL_ID)
	}

	fn dollar(amount: u128) -> u128 {
		amount.saturating_mul(Price::accuracy())
	}

	#[test]
	fn test_buy_and_sell() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(dollar(10_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(10)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(liquidity(), dollar(10_000));
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// synthetic = collateral / ask_price
				// 1650 ≈ 5000 / (3 * (1 + 0.01))
				//assert_eq!(synthetic_balance(&ALICE::get()), dollar(1650));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				// additional_collateral = (synthetic * price) * (1 + ratio) - collateral
				// 445 = (1650 * 3.0) * (1 + 0.1) - 5000
				// 5000 = ALICE -> ModuleTokens
				// 445 = LiquidityPool -> ModuleTokens
				//assert_eq!(synthetic_collateral_balance(), dollar(5445));
				assert_eq!(synthetic_collateral_balance(), 5445544554455445544553);
				// collateralise = balance - additional_collateral
				// 9555 = 10_000 - 445
				//assert_eq!(liquidity(), dollar(9555));
				assert_eq!(liquidity(), 9554455445544554455447);

				assert_ok!(sell(&ALICE::get(), FEUR, dollar(800)));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 850165016501650165016);
				// collateral = synthetic * bid_price
				// 2376 = 800 * (3 * (1 - 0.01))
				assert_eq!(collateral_balance(&ALICE::get()), dollar(7376));
				// redeem_collateral = collateral_position - (synthetic * price) * (1 + ratio)
				// 2805 = (850 * 3) * (1 + 0.1)
				assert_eq!(synthetic_collateral_balance(), 2805544554455445544553);
				// 2376 = ModuleTokens -> ALICE
				// 264 = 5445 - 2805 - 2376
				// 264 = ModuleTokens -> LiquidityPool
				// 9819 = 9555 + 264
				assert_eq!(liquidity(), 9818455445544554455447);
			});
	}

	#[test]
	fn test_buy_all_of_collateral() {
		ExtBuilder::default()
			.balances(vec![(POOL::get(), AUSD, 1000), (ALICE::get(), AUSD, 1000)])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(1000));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(100)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(1, 1))
				]));

				assert_eq!(collateral_balance(&ALICE::get()), 1000);
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(liquidity(), 1000);
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_ok!(buy(&ALICE::get(), FEUR, 1000));
				// synthetic = collateral / ask_price
				// 990 ≈ 1000 / (1 * (1 + 0.01))
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 990);
				// balance = balance - (synthetic * ask_price)
				// 0 ≈ 1000 - (990 * 1.01)
				assert_eq!(collateral_balance(&ALICE::get()), 0);
				// additional_collateral = (synthetic * price) * (1 + ratio) - collateral
				// 980  = (990 * 1.0) * (1 + 1) - 1000
				// 1000 = ALICE -> ModuleTokens
				// 980 = LiquidityPool -> ModuleTokens
				assert_eq!(synthetic_collateral_balance(), 1980);
				// collateralise = balance - additional_collateral
				// 20 = 1000 - 980
				assert_eq!(liquidity(), 20);

				assert_ok!(sell(&ALICE::get(), FEUR, 990));
				// synthetic balance is 190, below ExistentialDeposit
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 0);
				// collateral = synthetic * bid_price
				// 980 = 990 * (1 * (1 - 0.01))
				assert_eq!(collateral_balance(&ALICE::get()), 980);
				// redeem_collateral = collateral_position - (synthetic * price) * (1 + ratio)
				// 0 = (0 * 1) * (1 + 0.1)
				assert_eq!(synthetic_collateral_balance(), 0);
				// 980 = ModuleTokens -> ALICE
				// 1000 = 1980 - 980
				// 1000 = ModuleTokens -> LiquidityPool
				// 1020 = 1000 + 20
				assert_eq!(liquidity(), 1020);
			});
	}

	#[test]
	fn test_take_profit() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(dollar(10_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(10)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(liquidity(), dollar(10_000));
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				//assert_eq!(synthetic_balance(&ALICE::get(), FEUR), dollar(1650));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				//assert_eq!(synthetic_collateral_balance(), dollar(5445));
				assert_eq!(synthetic_collateral_balance(), 5445544554455445544553);
				//assert_eq!(liquidity(), dollar(9555));
				assert_eq!(liquidity(), 9554455445544554455447);

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(31, 10))]));

				assert_ok!(sell(&ALICE::get(), FEUR, synthetic_balance(&ALICE::get(), FEUR)));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(collateral_balance(&ALICE::get()), 10064356435643564356434);
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_eq!(liquidity(), 9935643564356435643566);
			});
	}

	#[test]
	fn test_stop_lost() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(dollar(10_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(10)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(liquidity(), dollar(10_000));
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				//assert_eq!(synthetic_balance(&ALICE::get(), FEUR), dollar(1650));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				//assert_eq!(synthetic_collateral_balance(), dollar(5445));
				assert_eq!(synthetic_collateral_balance(), 5445544554455445544553);
				//assert_eq!(liquidity(), dollar(9555));
				assert_eq!(liquidity(), 9554455445544554455447);

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(2, 1))]));

				assert_ok!(sell(&ALICE::get(), FEUR, synthetic_balance(&ALICE::get(), FEUR)));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(collateral_balance(&ALICE::get()), 8267326732673267326731);
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_eq!(liquidity(), 11732673267326732673269);
			});
	}

	#[test]
	fn test_multiple_users() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
				(BOB::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(10)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(10_000));
				assert_eq!(liquidity(), dollar(20_000));
				assert_eq!(synthetic_collateral_balance(), 0);

				// ALICE buy synthetic
				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				assert_eq!(synthetic_collateral_balance(), 5445544554455445544553);
				assert_eq!(liquidity(), 19554455445544554455447);

				// BOB buy synthetic
				assert_ok!(buy(&BOB::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&BOB::get()), dollar(5000));
				assert_eq!(synthetic_balance(&BOB::get(), FEUR), 1650165016501650165016);
				assert_eq!(synthetic_collateral_balance(), 10891089108910891089106);
				assert_eq!(liquidity(), 19108910891089108910894);

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(2, 1))]));

				// ALICE buy synthetic and BOB sell synthetic
				assert_ok!(buy(&ALICE::get(), FEUR, dollar(2000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(3000));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 2640264026402640264025);
				assert_eq!(synthetic_collateral_balance(), 13069306930693069306926);
				assert_eq!(liquidity(), 18930693069306930693074);
				assert_ok!(sell(&BOB::get(), FEUR, dollar(1000)));
				assert_eq!(collateral_balance(&BOB::get()), 6980000000000000000000);
				assert_eq!(synthetic_balance(&BOB::get(), FEUR), 650165016501650165016);
				assert_eq!(synthetic_collateral_balance(), 7238943894389438943890);
				assert_eq!(liquidity(), 22781056105610561056110);

				// ALICE sell synthetic and BOB buy synthetic
				assert_ok!(sell(&ALICE::get(), FEUR, dollar(1000)));
				assert_eq!(collateral_balance(&ALICE::get()), 4980000000000000000000);
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 1640264026402640264025);
				assert_eq!(synthetic_collateral_balance(), 5038943894389438943890);
				assert_eq!(liquidity(), 23001056105610561056110);
				assert_ok!(buy(&BOB::get(), FEUR, dollar(2000)));
				assert_eq!(collateral_balance(&BOB::get()), 4980000000000000000000);
				assert_eq!(synthetic_balance(&BOB::get(), FEUR), 1640264026402640264025);
				assert_eq!(synthetic_collateral_balance(), 7217161716171617161710);
				assert_eq!(liquidity(), 22822838283828382838290);

				assert_ok!(sell(&ALICE::get(), FEUR, synthetic_balance(&ALICE::get(), FEUR)));
				assert_ok!(sell(&BOB::get(), FEUR, synthetic_balance(&BOB::get(), FEUR)));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(collateral_balance(&ALICE::get()), 8227722772277227722769);
				assert_eq!(synthetic_balance(&BOB::get(), FEUR), 0);
				assert_eq!(collateral_balance(&BOB::get()), 8227722772277227722769);
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_eq!(liquidity(), 23544554455445544554462);
			});
	}

	#[test]
	fn test_multiple_users_multiple_currencies() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(40_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
				(BOB::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(synthetic_enabled(FJPY, true));
				assert_ok!(deposit_liquidity(dollar(40_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FJPY, Permill::from_percent(10)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_spread(FJPY, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1)),
					(FJPY, Price::from_rational(4, 1))
				]));

				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(10_000));
				assert_eq!(liquidity(), dollar(40_000));
				assert_eq!(synthetic_collateral_balance(), 0);

				// ALICE buy synthetic FEUR and BOB buy synthetic FJPY
				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				assert_eq!(synthetic_collateral_balance(), 5445544554455445544553);
				assert_eq!(liquidity(), 39554455445544554455447);

				assert_ok!(buy(&BOB::get(), FJPY, dollar(5000)));
				assert_eq!(collateral_balance(&BOB::get()), dollar(5000));
				assert_eq!(synthetic_balance(&BOB::get(), FJPY), 1237623762376237623762);
				assert_eq!(synthetic_collateral_balance(), 10891089108910891089106);
				assert_eq!(liquidity(), 39108910891089108910894);

				// change price
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(2, 1))]));
				assert_ok!(set_oracle_price(vec![(FJPY, Price::from_rational(5, 1))]));

				// ALICE buy synthetic FJPY and BOB sell FEUR
				assert_ok!(buy(&ALICE::get(), FJPY, dollar(2000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(3000));
				assert_eq!(synthetic_balance(&ALICE::get(), FJPY), 396039603960396039603);
				assert_eq!(synthetic_collateral_balance(), 13069306930693069306922);
				assert_eq!(liquidity(), 38930693069306930693078);

				assert_ok!(buy(&BOB::get(), FEUR, dollar(2000)));
				assert_eq!(collateral_balance(&BOB::get()), dollar(3000));
				assert_eq!(synthetic_balance(&BOB::get(), FEUR), 990099009900990099009);
				assert_eq!(synthetic_collateral_balance(), 15247524752475247524742);
				assert_eq!(liquidity(), 38752475247524752475258);

				// ALICE sell synthetic FEUR and BOB sell synthetic FJPY
				assert_ok!(sell(&ALICE::get(), FEUR, dollar(100)));
				assert_eq!(collateral_balance(&ALICE::get()), 3198000000000000000000);
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 1550165016501650165016);
				assert_eq!(synthetic_collateral_balance(), 13212343234323432343224);
				assert_eq!(liquidity(), 40589656765676567656776);

				assert_ok!(sell(&BOB::get(), FJPY, dollar(100)));
				assert_eq!(collateral_balance(&BOB::get()), 3495000000000000000000);
				assert_eq!(synthetic_balance(&BOB::get(), FJPY), 1137623762376237623762);
				assert_eq!(synthetic_collateral_balance(), 12717343234323432343224);
				assert_eq!(liquidity(), 40589656765676567656776);

				// ALICE sell synthetic FJPY and BOB sell synthetic FEUR
				assert_ok!(sell(&ALICE::get(), FJPY, dollar(100)));
				assert_eq!(collateral_balance(&ALICE::get()), 3693000000000000000000);
				assert_eq!(synthetic_balance(&ALICE::get(), FJPY), 296039603960396039603);
				assert_eq!(synthetic_collateral_balance(), 12222343234323432343224);
				assert_eq!(liquidity(), 40589656765676567656776);

				assert_ok!(sell(&BOB::get(), FEUR, dollar(100)));
				assert_eq!(collateral_balance(&BOB::get()), 3693000000000000000000);
				assert_eq!(synthetic_balance(&BOB::get(), FEUR), 890099009900990099009);
				assert_eq!(synthetic_collateral_balance(), 12002343234323432343224);
				assert_eq!(liquidity(), 40611656765676567656776);
			});
	}

	#[test]
	fn test_liquidate_position() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(10)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));

				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(300, 90))]));

				assert_ok!(liquidate(&ALICE::get(), FEUR, synthetic_balance(&ALICE::get(), FEUR)));
				assert_eq!(liquidity(), 19554455445544554455447);
				assert_eq!(collateral_balance(&ALICE::get()), 10445544554455445544552);
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_eq!(liquidity(), 19554455445544554455447);
			});
	}

	#[test]
	fn test_add_collateral() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(40_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(1)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));

				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(300, 90))]));

				assert_ok!(liquidate(&ALICE::get(), FEUR, 1));
				assert_ok!(add_collateral(&POOL::get(), FEUR, dollar(20_000)));
				assert_noop!(
					liquidate(&ALICE::get(), FEUR, 1),
					synthetic_protocol::Error::<Runtime>::StillInSafePosition
				);
			});
	}

	#[test]
	fn test_liquidate_partially() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(10)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));

				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(300, 90))]));

				assert_ok!(liquidate(&ALICE::get(), FEUR, dollar(800)));
				assert_eq!(collateral_balance(&ALICE::get()), 7640000000000000000000);
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 850165016501650165016);
				assert_eq!(synthetic_collateral_balance(), 2805544554455445544553);
				assert_eq!(liquidity(), 19554455445544554455447);

				assert_ok!(liquidate(&ALICE::get(), FEUR, synthetic_balance(&ALICE::get(), FEUR)));
				assert_eq!(collateral_balance(&ALICE::get()), 10445544554455445544552);
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(synthetic_collateral_balance(), 0);
				assert_eq!(liquidity(), 19554455445544554455447);
			});
	}

	#[test]
	fn test_liquidate_remove() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(set_enabled_trades());
				assert_ok!(synthetic_enabled(FEUR, true));
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(FEUR, Permill::from_percent(10)));
				assert_ok!(set_spread(FEUR, Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));

				assert_ok!(buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				assert_noop!(
					remove_pool(&POOL::get()),
					synthetic_liquidity_pools::Error::<Runtime>::CannotRemovePool
				);

				assert_ok!(sell(&ALICE::get(), FEUR, synthetic_balance(&ALICE::get(), FEUR)));
				assert_eq!(synthetic_balance(&ALICE::get(), FEUR), 0);
				assert_ok!(remove_pool(&POOL::get()));
			});
	}
}
