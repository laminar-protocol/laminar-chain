/// tests for this module

#[cfg(test)]

mod tests {
	use frame_support::{assert_noop, assert_ok};
	pub use laminar_runtime::{AccountId, CurrencyId, LiquidityPoolId, Runtime};
	pub use module_primitives::{Balance, Leverage, Leverages};
	pub use orml_prices::Price;
	use orml_traits::{BasicCurrency, MultiCurrency};
	pub use sp_runtime::{traits::Zero, DispatchResult, Perbill, Permill};

	pub fn origin_of(account_id: AccountId) -> <Runtime as system::Trait>::Origin {
		<Runtime as system::Trait>::Origin::signed(account_id)
	}

	pub type ModuleProtocol = synthetic_protocol::Module<Runtime>;
	pub type ModuleTokens = synthetic_tokens::Module<Runtime>;
	pub type ModuleOracle = orml_oracle::Module<Runtime>;
	pub type ModuleLiquidityPools = liquidity_pools::Module<Runtime>;

	const LIQUIDITY_POOL_ID: LiquidityPoolId = 0;

	const ORACLE1: [u8; 32] = [0u8; 32];
	const ORACLE2: [u8; 32] = [1u8; 32];
	const ORACLE3: [u8; 32] = [2u8; 32];

	const POOL: [u8; 32] = [3u8; 32];
	const ALICE: [u8; 32] = [4u8; 32];
	const BOB: [u8; 32] = [5u8; 32];

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
				members: vec![
					AccountId::from(ORACLE1),
					AccountId::from(ORACLE2),
					AccountId::from(ORACLE3),
				],
				phantom: Default::default(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			t.into()
		}
	}

	pub fn create_pool() -> DispatchResult {
		ModuleLiquidityPools::create_pool(origin_of(AccountId::from(POOL)))?;
		ModuleLiquidityPools::set_enabled_trades(
			origin_of(AccountId::from(POOL)),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			Leverages::all(),
		)
	}

	pub fn deposit_liquidity(amount: Balance) -> DispatchResult {
		ModuleLiquidityPools::deposit_liquidity(origin_of(AccountId::from(POOL)), LIQUIDITY_POOL_ID, amount)
	}

	pub fn set_min_additional_collateral_ratio(permill: Permill) -> DispatchResult {
		ModuleLiquidityPools::set_min_additional_collateral_ratio(<Runtime as system::Trait>::Origin::ROOT, permill)
	}

	pub fn set_additional_collateral_ratio(permill: Permill) -> DispatchResult {
		ModuleLiquidityPools::set_additional_collateral_ratio(
			origin_of(AccountId::from(POOL)),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			Some(permill),
		)
	}

	pub fn set_spread(permill: Permill) -> DispatchResult {
		ModuleLiquidityPools::set_spread(
			origin_of(AccountId::from(POOL)),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			permill,
			permill,
		)
	}

	fn set_oracle_price(prices: Vec<(CurrencyId, Price)>) -> DispatchResult {
		prices.iter().for_each(|(c, p)| {
			assert_ok!(ModuleOracle::feed_value(origin_of(AccountId::from(ORACLE1)), *c, *p));
			assert_ok!(ModuleOracle::feed_value(origin_of(AccountId::from(ORACLE2)), *c, *p));
			assert_ok!(ModuleOracle::feed_value(origin_of(AccountId::from(ORACLE3)), *c, *p));
		});
		Ok(())
	}

	fn buy(who: &AccountId, amount: Balance) -> DispatchResult {
		ModuleProtocol::mint(
			origin_of(who.clone()),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			amount,
			Permill::from_percent(10),
		)
	}

	fn sell(who: &AccountId, amount: Balance) -> DispatchResult {
		ModuleProtocol::redeem(
			origin_of(who.clone()),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			amount,
			Permill::from_percent(10),
		)
	}

	fn collateral_balance(who: &AccountId) -> Balance {
		<Runtime as synthetic_protocol::Trait>::CollateralCurrency::balance(who)
	}

	fn synthetic_balance(who: &AccountId) -> Balance {
		<Runtime as synthetic_protocol::Trait>::MultiCurrency::balance(CurrencyId::FEUR, &who)
	}

	fn liquidity() -> Balance {
		ModuleLiquidityPools::balances(LIQUIDITY_POOL_ID)
	}

	fn add_collateral(who: &AccountId, amount: Balance) -> DispatchResult {
		ModuleProtocol::add_collateral(origin_of(who.clone()), LIQUIDITY_POOL_ID, CurrencyId::FEUR, amount)
	}

	fn liquidate(who: &AccountId, amount: Balance) -> DispatchResult {
		ModuleProtocol::liquidate(origin_of(who.clone()), LIQUIDITY_POOL_ID, CurrencyId::FEUR, amount)
	}

	fn remove_pool(who: &AccountId) -> DispatchResult {
		ModuleLiquidityPools::remove_pool(origin_of(who.clone()), LIQUIDITY_POOL_ID)
	}

	fn dollar(amount: u128) -> u128 {
		amount.saturating_mul(Price::accuracy())
	}

	#[test]
	fn test_buy_and_sell() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, dollar(10_000)),
				(AccountId::from(ALICE), CurrencyId::AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(dollar(10_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(10_000));
				assert_eq!(collateral_balance(&AccountId::from(POOL)), 0);
				assert_eq!(liquidity(), dollar(10_000));
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);
				assert_ok!(buy(&AccountId::from(ALICE), dollar(5000)));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(5000));
				// synthetic = collateral / ask_price
				// 1650 ≈ 5000 / (3 * (1 + 0.01))
				//assert_eq!(synthetic_balance(&AccountId::from(ALICE)), dollar(1650));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 1650165016501650165016);
				// additional_collateral = (synthetic * price) * (1 + ratio) - collateral
				// 445 = (1650 * 3.0) * (1 + 0.1) - 5000
				// 5000 = ALICE -> ModuleTokens
				// 445 = LiquidityPool -> ModuleTokens
				//assert_eq!(collateral_balance(&ModuleTokens::account_id()), dollar(5445));
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 5445544554455445544553);
				// collateralise = balance - additional_collateral
				// 9555 = 10_000 - 445
				//assert_eq!(liquidity(), dollar(9555));
				assert_eq!(liquidity(), 9554455445544554455447);

				assert_ok!(sell(&AccountId::from(ALICE), dollar(800)));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 850165016501650165016);
				// collateral = synthetic * bid_price
				// 2376 = 800 * (3 * (1 - 0.01))
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(7376));
				// redeem_collateral = collateral_position - (synthetic * price) * (1 + ratio)
				// 2805 = (850 * 3) * (1 + 0.1)
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 2805544554455445544553);
				// 2376 = ModuleTokens -> ALICE
				// 264 = 5445 - 2805 - 2376
				// 264 = ModuleTokens -> LiquidityPool
				// 9819 = 9555 + 264
				assert_eq!(liquidity(), 9818455445544554455447);
			});
	}

	#[test]
	fn test_take_profit() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, dollar(10_000)),
				(AccountId::from(ALICE), CurrencyId::AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(dollar(10_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(10_000));
				assert_eq!(collateral_balance(&AccountId::from(POOL)), 0);
				assert_eq!(liquidity(), dollar(10_000));
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);
				assert_ok!(buy(&AccountId::from(ALICE), dollar(5000)));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(5000));
				//assert_eq!(synthetic_balance(&AccountId::from(ALICE)), dollar(1650));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 1650165016501650165016);
				//assert_eq!(collateral_balance(&ModuleTokens::account_id()), dollar(5445));
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 5445544554455445544553);
				//assert_eq!(liquidity(), dollar(9555));
				assert_eq!(liquidity(), 9554455445544554455447);

				assert_ok!(set_oracle_price(vec![(CurrencyId::FEUR, Price::from_rational(31, 10))]));

				assert_ok!(sell(
					&AccountId::from(ALICE),
					synthetic_balance(&AccountId::from(ALICE))
				));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 0);
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 10064356435643564356434);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);
				assert_eq!(liquidity(), 9935643564356435643566);
			});
	}

	#[test]
	fn test_stop_lost() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, dollar(10_000)),
				(AccountId::from(ALICE), CurrencyId::AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(dollar(10_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(10_000));
				assert_eq!(collateral_balance(&AccountId::from(POOL)), 0);
				assert_eq!(liquidity(), dollar(10_000));
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);
				assert_ok!(buy(&AccountId::from(ALICE), dollar(5000)));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(5000));
				//assert_eq!(synthetic_balance(&AccountId::from(ALICE)), dollar(1650));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 1650165016501650165016);
				//assert_eq!(collateral_balance(&ModuleTokens::account_id()), dollar(5445));
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 5445544554455445544553);
				//assert_eq!(liquidity(), dollar(9555));
				assert_eq!(liquidity(), 9554455445544554455447);

				assert_ok!(set_oracle_price(vec![(CurrencyId::FEUR, Price::from_rational(2, 1))]));

				assert_ok!(sell(
					&AccountId::from(ALICE),
					synthetic_balance(&AccountId::from(ALICE))
				));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 0);
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 8267326732673267326731);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);
				assert_eq!(liquidity(), 11732673267326732673269);
			});
	}

	#[test]
	fn test_multiple_users() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, dollar(20_000)),
				(AccountId::from(ALICE), CurrencyId::AUSD, dollar(10_000)),
				(AccountId::from(BOB), CurrencyId::AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&AccountId::from(POOL)), 0);
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(10_000));
				assert_eq!(collateral_balance(&AccountId::from(BOB)), dollar(10_000));
				assert_eq!(liquidity(), dollar(20_000));
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);

				// ALICE buy synthetic
				assert_ok!(buy(&AccountId::from(ALICE), dollar(5000)));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(5000));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 1650165016501650165016);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 5445544554455445544553);
				assert_eq!(liquidity(), 19554455445544554455447);

				// BOB buy synthetic
				assert_ok!(buy(&AccountId::from(BOB), dollar(5000)));
				assert_eq!(collateral_balance(&AccountId::from(BOB)), dollar(5000));
				assert_eq!(synthetic_balance(&AccountId::from(BOB)), 1650165016501650165016);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 10891089108910891089106);
				assert_eq!(liquidity(), 19108910891089108910894);

				assert_ok!(set_oracle_price(vec![(CurrencyId::FEUR, Price::from_rational(2, 1))]));

				// ALICE buy synthetic and BOB sell synthetic
				assert_ok!(buy(&AccountId::from(ALICE), dollar(2000)));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), dollar(3000));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 2640264026402640264025);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 13069306930693069306926);
				assert_eq!(liquidity(), 18930693069306930693074);
				assert_ok!(sell(&AccountId::from(BOB), dollar(1000)));
				assert_eq!(collateral_balance(&AccountId::from(BOB)), 6980000000000000000000);
				assert_eq!(synthetic_balance(&AccountId::from(BOB)), 650165016501650165016);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 7238943894389438943890);
				assert_eq!(liquidity(), 22781056105610561056110);

				// ALICE sell synthetic and BOB buy synthetic
				assert_ok!(sell(&AccountId::from(ALICE), dollar(1000)));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 4980000000000000000000);
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 1640264026402640264025);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 5038943894389438943890);
				assert_eq!(liquidity(), 23001056105610561056110);
				assert_ok!(buy(&AccountId::from(BOB), dollar(2000)));
				assert_eq!(collateral_balance(&AccountId::from(BOB)), 4980000000000000000000);
				assert_eq!(synthetic_balance(&AccountId::from(BOB)), 1640264026402640264025);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 7217161716171617161710);
				assert_eq!(liquidity(), 22822838283828382838290);

				assert_ok!(sell(
					&AccountId::from(ALICE),
					synthetic_balance(&AccountId::from(ALICE))
				));
				assert_ok!(sell(&AccountId::from(BOB), synthetic_balance(&AccountId::from(BOB))));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 0);
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 8227722772277227722769);
				assert_eq!(synthetic_balance(&AccountId::from(BOB)), 0);
				assert_eq!(collateral_balance(&AccountId::from(BOB)), 8227722772277227722769);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);
				assert_eq!(liquidity(), 23544554455445544554462);
			});
	}

	#[test]
	fn test_liquidate_position() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, dollar(20_000)),
				(AccountId::from(ALICE), CurrencyId::AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_ok!(buy(&AccountId::from(ALICE), dollar(5000)));

				assert_ok!(set_oracle_price(vec![(
					CurrencyId::FEUR,
					Price::from_rational(300, 90)
				)]));

				assert_ok!(liquidate(
					&AccountId::from(ALICE),
					synthetic_balance(&AccountId::from(ALICE))
				));
				assert_eq!(liquidity(), 19554455445544554455447);
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 10445544554455445544552);
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 0);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);
				assert_eq!(liquidity(), 19554455445544554455447);
			});
	}

	#[test]
	fn test_add_collateral() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, dollar(40_000)),
				(AccountId::from(ALICE), CurrencyId::AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(1)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_ok!(buy(&AccountId::from(ALICE), dollar(5000)));

				assert_ok!(set_oracle_price(vec![(
					CurrencyId::FEUR,
					Price::from_rational(300, 90)
				)]));

				assert_ok!(liquidate(&AccountId::from(ALICE), 1));
				assert_ok!(add_collateral(&AccountId::from(POOL), dollar(20_000)));
				assert_noop!(
					liquidate(&AccountId::from(ALICE), 1),
					synthetic_protocol::Error::<Runtime>::StillInSafePosition
				);
			});
	}

	#[test]
	fn test_liquidate_partially() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, dollar(20_000)),
				(AccountId::from(ALICE), CurrencyId::AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_ok!(buy(&AccountId::from(ALICE), dollar(5000)));

				assert_ok!(set_oracle_price(vec![(
					CurrencyId::FEUR,
					Price::from_rational(300, 90)
				)]));

				assert_ok!(liquidate(&AccountId::from(ALICE), dollar(800)));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 7640000000000000000000);
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 850165016501650165016);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 2805544554455445544553);
				assert_eq!(liquidity(), 19554455445544554455447);

				assert_ok!(liquidate(
					&AccountId::from(ALICE),
					synthetic_balance(&AccountId::from(ALICE))
				));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 10445544554455445544552);
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 0);
				assert_eq!(collateral_balance(&ModuleTokens::account_id()), 0);
				assert_eq!(liquidity(), 19554455445544554455447);
			});
	}

	#[test]
	fn test_liquidate_remove() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, dollar(20_000)),
				(AccountId::from(ALICE), CurrencyId::AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(dollar(20_000)));
				assert_ok!(set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					// collateral price set to `1` for calculation simplicity.
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_ok!(buy(&AccountId::from(ALICE), dollar(5000)));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 1650165016501650165016);
				assert_noop!(
					remove_pool(&AccountId::from(POOL)),
					liquidity_pools::Error::<Runtime>::CannotRemovePool
				);

				assert_ok!(sell(
					&AccountId::from(ALICE),
					synthetic_balance(&AccountId::from(ALICE))
				));
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 0);
				assert_ok!(remove_pool(&AccountId::from(POOL)));
			});
	}
}
