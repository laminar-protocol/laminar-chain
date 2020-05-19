/// tests for this module

#[cfg(test)]

mod tests {
	use frame_support::{assert_noop, assert_ok};
	use laminar_runtime::{
		tests::*,
		BaseLiquidityPoolsSyntheticInstance,
		CurrencyId::{AUSD, FEUR, FJPY},
		Runtime,
	};
	use orml_prices::Price;
	use orml_utilities::FixedU128;
	use sp_runtime::Permill;
	use synthetic_protocol_rpc_runtime_api::PoolInfo;

	#[test]
	fn test_synthetic_buy_and_sell() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(3, 1))]));

				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(synthetic_liquidity(), dollar(10_000));
				assert_eq!(
					synthetic_pool_info(FEUR),
					Some(PoolInfo {
						collateral_ratio: FixedU128::zero(),
						is_safe: false
					})
				);
				assert_eq!(synthetic_balance(), 0);
				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// synthetic = collateral / ask_price
				// 1650 ≈ 5000 / (3 * (1 + 0.01))
				//assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), dollar(1650));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				// additional_collateral = (synthetic * price) * (1 + ratio) - collateral
				// 445 = (1650 * 3.0) * (1 + 0.1) - 5000
				// 5000 = ALICE -> ModuleTokens
				// 445 = LiquidityPool -> ModuleTokens
				//assert_eq!(synthetic_balance(), dollar(5445));
				assert_eq!(synthetic_balance(), 5445544554455445544553);
				// collateralise = balance - additional_collateral
				// 9555 = 10_000 - 445
				//assert_eq!(liquidity(), dollar(9555));
				assert_eq!(synthetic_liquidity(), 9554455445544554455447);
				assert_eq!(
					synthetic_pool_info(FEUR),
					Some(PoolInfo {
						collateral_ratio: FixedU128::from_rational(11, 10),
						is_safe: true
					})
				);

				assert_ok!(synthetic_sell(&ALICE::get(), FEUR, dollar(800)));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 850165016501650165016);
				// collateral = synthetic * bid_price
				// 2376 = 800 * (3 * (1 - 0.01))
				assert_eq!(collateral_balance(&ALICE::get()), dollar(7376));
				// redeem_collateral = collateral_position - (synthetic * price) * (1 + ratio)
				// 2805 = (850 * 3) * (1 + 0.1)
				assert_eq!(synthetic_balance(), 2805544554455445544553);
				// 2376 = ModuleTokens -> ALICE
				// 264 = 5445 - 2805 - 2376
				// 264 = ModuleTokens -> LiquidityPool
				// 9819 = 9555 + 264
				assert_eq!(synthetic_liquidity(), 9818455445544554455447);
				assert_eq!(
					synthetic_pool_info(FEUR),
					Some(PoolInfo {
						collateral_ratio: FixedU128::from_rational(11, 10),
						is_safe: true
					})
				);
			});
	}

	#[test]
	fn test_synthetic_buy_all_of_collateral() {
		ExtBuilder::default()
			.balances(vec![(POOL::get(), AUSD, 1000), (ALICE::get(), AUSD, 1000)])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), 1000));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(100)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(1)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(1, 1))]));

				assert_eq!(collateral_balance(&ALICE::get()), 1000);
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(synthetic_liquidity(), 1000);
				assert_eq!(synthetic_balance(), 0);
				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, 1000));
				// synthetic = collateral / ask_price
				// 990 ≈ 1000 / (1 * (1 + 0.01))
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 990);
				// balance = balance - (synthetic * ask_price)
				// 0 ≈ 1000 - (990 * 1.01)
				assert_eq!(collateral_balance(&ALICE::get()), 0);
				// additional_collateral = (synthetic * price) * (1 + ratio) - collateral
				// 980  = (990 * 1.0) * (1 + 1) - 1000
				// 1000 = ALICE -> ModuleTokens
				// 980 = LiquidityPool -> ModuleTokens
				assert_eq!(synthetic_balance(), 1980);
				// collateralise = balance - additional_collateral
				// 20 = 1000 - 980
				assert_eq!(synthetic_liquidity(), 20);

				assert_ok!(synthetic_sell(&ALICE::get(), FEUR, 990));
				// synthetic balance is 190, below ExistentialDeposit
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 0);
				// collateral = synthetic * bid_price
				// 980 = 990 * (1 * (1 - 0.01))
				assert_eq!(collateral_balance(&ALICE::get()), 980);
				// redeem_collateral = collateral_position - (synthetic * price) * (1 + ratio)
				// 0 = (0 * 1) * (1 + 0.1)
				assert_eq!(synthetic_balance(), 0);
				// 980 = ModuleTokens -> ALICE
				// 1000 = 1980 - 980
				// 1000 = ModuleTokens -> LiquidityPool
				// 1020 = 1000 + 20
				assert_eq!(synthetic_liquidity(), 1020);
			});
	}

	#[test]
	fn test_synthetic_trader_take_profit() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(3, 1))]));

				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(synthetic_liquidity(), dollar(10_000));
				assert_eq!(synthetic_balance(), 0);
				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				//assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), dollar(1650));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				//assert_eq!(synthetic_balance(), dollar(5445));
				assert_eq!(synthetic_balance(), 5445544554455445544553);
				//assert_eq!(synthetic_liquidity(), dollar(9555));
				assert_eq!(synthetic_liquidity(), 9554455445544554455447);

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(31, 10))]));

				assert_ok!(synthetic_sell(
					&ALICE::get(),
					FEUR,
					multi_currency_balance(&ALICE::get(), FEUR)
				));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(collateral_balance(&ALICE::get()), 10066006600660066006599);
				assert_eq!(synthetic_balance(), 0);
				assert_eq!(synthetic_liquidity(), 9933993399339933993401);
			});
	}

	#[test]
	fn test_synthetic_trader_stop_lost() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(3, 1))]));

				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(synthetic_liquidity(), dollar(10_000));
				assert_eq!(synthetic_balance(), 0);
				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				//assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), dollar(1650));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				//assert_eq!(synthetic_balance(), dollar(5445));
				assert_eq!(synthetic_balance(), 5445544554455445544553);
				//assert_eq!(synthetic_liquidity(), dollar(9555));
				assert_eq!(synthetic_liquidity(), 9554455445544554455447);

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(2, 1))]));

				assert_ok!(synthetic_sell(
					&ALICE::get(),
					FEUR,
					multi_currency_balance(&ALICE::get(), FEUR)
				));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(collateral_balance(&ALICE::get()), 8250825082508250825081);
				assert_eq!(synthetic_balance(), 0);
				assert_eq!(synthetic_liquidity(), 11749174917491749174919);
			});
	}

	#[test]
	fn test_synthetic_multiple_users() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
				(BOB::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(3, 1))]));

				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(10_000));
				assert_eq!(synthetic_liquidity(), dollar(20_000));
				assert_eq!(synthetic_balance(), 0);

				// ALICE buy synthetic
				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				assert_eq!(synthetic_balance(), 5445544554455445544553);
				assert_eq!(synthetic_liquidity(), 19554455445544554455447);

				// BOB buy synthetic
				assert_ok!(synthetic_buy(&BOB::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&BOB::get()), dollar(5000));
				assert_eq!(multi_currency_balance(&BOB::get(), FEUR), 1650165016501650165016);
				assert_eq!(synthetic_balance(), 10891089108910891089106);
				assert_eq!(synthetic_liquidity(), 19108910891089108910894);

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(2, 1))]));

				// ALICE buy synthetic and BOB sell synthetic
				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(2000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(3000));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 2635386691378497455656);
				assert_eq!(synthetic_balance(), 13058576793639955128514);
				assert_eq!(synthetic_liquidity(), 18941423206360044871486);
				assert_ok!(synthetic_sell(&BOB::get(), FEUR, dollar(1000)));
				assert_eq!(collateral_balance(&BOB::get()), 6970000000000000000000);
				assert_eq!(multi_currency_balance(&BOB::get(), FEUR), 650165016501650165016);
				assert_eq!(synthetic_balance(), 7228213757336324765478);
				assert_eq!(synthetic_liquidity(), 22801786242663675234522);

				// ALICE sell synthetic and BOB buy synthetic
				assert_ok!(synthetic_sell(&ALICE::get(), FEUR, dollar(1000)));
				assert_eq!(collateral_balance(&ALICE::get()), 4970000000000000000000);
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 1635386691378497455656);
				assert_eq!(synthetic_balance(), 5028213757336324765478);
				assert_eq!(synthetic_liquidity(), 23031786242663675234522);
				assert_ok!(synthetic_buy(&BOB::get(), FEUR, dollar(2000)));
				assert_eq!(collateral_balance(&BOB::get()), 4970000000000000000000);
				assert_eq!(multi_currency_balance(&BOB::get(), FEUR), 1635386691378497455656);
				assert_eq!(synthetic_balance(), 7195701442065388804886);
				assert_eq!(synthetic_liquidity(), 22864298557934611195114);

				assert_ok!(synthetic_sell(
					&ALICE::get(),
					FEUR,
					multi_currency_balance(&ALICE::get(), FEUR)
				));
				assert_ok!(synthetic_sell(
					&BOB::get(),
					FEUR,
					multi_currency_balance(&BOB::get(), FEUR)
				));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(collateral_balance(&ALICE::get()), 8191711782015639987642);
				assert_eq!(multi_currency_balance(&BOB::get(), FEUR), 0);
				assert_eq!(collateral_balance(&BOB::get()), 8191711782015639987642);
				assert_eq!(synthetic_balance(), 0);
				assert_eq!(synthetic_liquidity(), 23616576435968720024716);
			});
	}

	#[test]
	fn test_synthetic_multiple_users_multiple_currencies() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(40_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
				(BOB::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(40_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FJPY,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(synthetic_set_spread(FJPY, cent(4)));
				assert_ok!(set_oracle_price(vec![
					(FEUR, Price::from_rational(3, 1)),
					(FJPY, Price::from_rational(4, 1))
				]));

				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(10_000));
				assert_eq!(synthetic_liquidity(), dollar(40_000));
				assert_eq!(synthetic_balance(), 0);

				// ALICE buy synthetic FEUR and BOB buy synthetic FJPY
				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				assert_eq!(synthetic_balance(), 5445544554455445544553);
				assert_eq!(synthetic_liquidity(), 39554455445544554455447);

				assert_ok!(synthetic_buy(&BOB::get(), FJPY, dollar(5000)));
				assert_eq!(collateral_balance(&BOB::get()), dollar(5000));
				assert_eq!(multi_currency_balance(&BOB::get(), FJPY), 1237623762376237623762);
				assert_eq!(synthetic_balance(), 10891089108910891089106);
				assert_eq!(synthetic_liquidity(), 39108910891089108910894);

				// change price
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(2, 1))]));
				assert_ok!(set_oracle_price(vec![(FJPY, Price::from_rational(5, 1))]));

				// ALICE buy synthetic FJPY and BOB sell FEUR
				assert_ok!(synthetic_buy(&ALICE::get(), FJPY, dollar(2000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(3000));
				assert_eq!(multi_currency_balance(&ALICE::get(), FJPY), 396825396825396825396);
				assert_eq!(synthetic_balance(), 13073628791450573628784);
				assert_eq!(synthetic_liquidity(), 38926371208549426371216);

				assert_ok!(synthetic_buy(&BOB::get(), FEUR, dollar(2000)));
				assert_eq!(collateral_balance(&BOB::get()), dollar(3000));
				assert_eq!(multi_currency_balance(&BOB::get(), FEUR), 985221674876847290640);
				assert_eq!(synthetic_balance(), 15241116476179637668192);
				assert_eq!(synthetic_liquidity(), 38758883523820362331808);

				// ALICE sell synthetic FEUR and BOB sell synthetic FJPY
				assert_ok!(synthetic_sell(&ALICE::get(), FEUR, dollar(100)));
				assert_eq!(collateral_balance(&ALICE::get()), 3197000000000000000000);
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 1550165016501650165016);
				assert_eq!(synthetic_balance(), 13205934958027822486674);
				assert_eq!(synthetic_liquidity(), 40597065041972177513326);

				assert_ok!(synthetic_sell(&BOB::get(), FJPY, dollar(100)));
				assert_eq!(collateral_balance(&BOB::get()), 3496000000000000000000);
				assert_eq!(multi_currency_balance(&BOB::get(), FJPY), 1137623762376237623762);
				assert_eq!(synthetic_balance(), 12709934958027822486674);
				assert_eq!(synthetic_liquidity(), 40597065041972177513326);

				// ALICE sell synthetic FJPY and BOB sell synthetic FEUR
				assert_ok!(synthetic_sell(&ALICE::get(), FJPY, dollar(100)));
				assert_eq!(collateral_balance(&ALICE::get()), 3693000000000000000000);
				assert_eq!(multi_currency_balance(&ALICE::get(), FJPY), 296825396825396825396);
				assert_eq!(synthetic_balance(), 12213934958027822486674);
				assert_eq!(synthetic_liquidity(), 40597065041972177513326);

				assert_ok!(synthetic_sell(&BOB::get(), FEUR, dollar(100)));
				assert_eq!(collateral_balance(&BOB::get()), 3693000000000000000000);
				assert_eq!(multi_currency_balance(&BOB::get(), FEUR), 885221674876847290640);
				assert_eq!(synthetic_balance(), 11993934958027822486674);
				assert_eq!(synthetic_liquidity(), 40620065041972177513326);
			});
	}

	#[test]
	fn test_synthetic_liquidate_position() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(3, 1))]));

				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(300, 95))]));

				assert_ok!(synthetic_liquidate(
					&ALICE::get(),
					FEUR,
					multi_currency_balance(&ALICE::get(), FEUR)
				));
				assert_eq!(synthetic_liquidity(), 19791123849227027966364);
				assert_eq!(collateral_balance(&ALICE::get()), 10208876150772972033636);
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(synthetic_balance(), 0);
			});
	}

	#[test]
	fn test_synthetic_add_collateral() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(40_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(1)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(3, 1))]));

				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(300, 90))]));

				assert_ok!(synthetic_liquidate(&ALICE::get(), FEUR, 1));
				assert_ok!(synthetic_add_collateral(&POOL::get(), FEUR, dollar(20_000)));
				assert_noop!(
					synthetic_liquidate(&ALICE::get(), FEUR, 1),
					synthetic_protocol::Error::<Runtime>::StillInSafePosition
				);
			});
	}

	#[test]
	fn test_synthetic_liquidate_partially() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(3, 1))]));

				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));

				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(300, 95))]));

				assert_ok!(synthetic_liquidate(&ALICE::get(), FEUR, dollar(800)));
				assert_eq!(collateral_balance(&ALICE::get()), 7525263157894736841931);
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 850165016501650165016);
				assert_eq!(synthetic_balance(), 2805544554455445544412);
				assert_eq!(synthetic_liquidity(), 19669192287649817613657);

				assert_ok!(synthetic_liquidate(
					&ALICE::get(),
					FEUR,
					multi_currency_balance(&ALICE::get(), FEUR)
				));
				assert_eq!(collateral_balance(&ALICE::get()), 10208876150772972038611);
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 0);
				assert_eq!(synthetic_balance(), 0);
				assert_eq!(synthetic_liquidity(), 19791123849227027961389);
				assert_ok!(synthetic_withdraw_liquidity(&POOL::get(), dollar(1000)));
			});
	}

	#[test]
	fn test_synthetic_liquidate_remove() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(synthetic_create_pool());
				assert_ok!(synthetic_set_enabled_trades());
				assert_ok!(synthetic_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(synthetic_set_min_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(synthetic_set_additional_collateral_ratio(
					FEUR,
					Permill::from_percent(10)
				));
				assert_ok!(synthetic_set_spread(FEUR, cent(3)));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(3, 1))]));

				assert_ok!(synthetic_buy(&ALICE::get(), FEUR, dollar(5000)));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 1650165016501650165016);
				assert_noop!(
					synthetic_remove_pool(&POOL::get()),
					base_liquidity_pools::Error::<Runtime, BaseLiquidityPoolsSyntheticInstance>::CannotRemovePool
				);

				assert_ok!(synthetic_sell(
					&ALICE::get(),
					FEUR,
					multi_currency_balance(&ALICE::get(), FEUR)
				));
				assert_eq!(multi_currency_balance(&ALICE::get(), FEUR), 0);
				assert_ok!(synthetic_disable_pool(&POOL::get()));
				assert_ok!(synthetic_remove_pool(&POOL::get()));
			});
	}
}
