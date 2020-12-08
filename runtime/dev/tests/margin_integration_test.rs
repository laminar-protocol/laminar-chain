/// tests for this module

#[cfg(test)]

mod tests {
	use dev_runtime::{
		tests::*,
		BaseLiquidityPoolsMarginInstance,
		CurrencyId::{AUSD, FEUR, FJPY},
		MaxSwap, Runtime, TreasuryAccount, DOLLARS,
	};
	use frame_support::{assert_noop, assert_ok};

	use margin_protocol_rpc_runtime_api::{MarginPoolState, MarginTraderState};
	use module_traits::MarginProtocolLiquidityPools;
	use primitives::{Leverage::*, Price};
	use sp_arithmetic::{FixedI128, FixedPointNumber};
	use sp_runtime::traits::{Bounded, CheckedAdd};

	#[test]
	fn test_margin_liquidity_pools() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(margin_create_pool());
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
					base_liquidity_pools::Error::<Runtime, BaseLiquidityPoolsMarginInstance>::NoPermission
				);

				assert_eq!(margin_pool_required_deposit(), fixed_i128_dollar(0));

				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.01)));
				assert_ok!(margin_set_max_spread(EUR_USD, Price::from_fraction(0.02)));
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.01)));

				assert_ok!(margin_set_enabled_trades());

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));
				assert_noop!(
					margin_set_swap_rate(
						EUR_USD,
						negative_one_percent(),
						MaxSwap::get().checked_add(&one_percent()).unwrap()
					),
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
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(3, 1))]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));
				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));
				assert_eq!(
					margin_pool_state(),
					Some(MarginPoolState {
						enp: FixedI128::max_value(),
						ell: FixedI128::max_value(),
						required_deposit: FixedI128::zero()
					})
				);
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(5000),
						margin_held: FixedI128::zero(),
						margin_level: FixedI128::max_value(),
						free_margin: FixedI128::saturating_from_integer(5000),
						unrealized_pl: FixedI128::zero()
					}
				);
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_eq!(
					margin_pool_state(),
					Some(MarginPoolState {
						enp: FixedI128::from_inner(0_686666666666666667),
						ell: FixedI128::from_inner(0_686666666666666667),
						required_deposit: FixedI128::zero()
					})
				);
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(4700),
						margin_held: FixedI128::saturating_from_integer(1515),
						margin_level: FixedI128::from_inner(310231023102310231),
						free_margin: FixedI128::saturating_from_integer(3185),
						unrealized_pl: FixedI128::saturating_from_integer(-300)
					}
				);

				assert_ok!(margin_close_position(
					&ALICE::get(),
					0,
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 + 0.03 = 3.03
				// close_price = 3 - 0.03 = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(4700));
				assert_eq!(margin_liquidity(), dollar(10_300));
				assert_eq!(
					margin_pool_state(),
					Some(MarginPoolState {
						enp: FixedI128::max_value(),
						ell: FixedI128::max_value(),
						required_deposit: FixedI128::zero()
					})
				);
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(4700),
						margin_held: FixedI128::zero(),
						margin_level: FixedI128::max_value(),
						free_margin: FixedI128::saturating_from_integer(4700),
						unrealized_pl: FixedI128::zero()
					}
				);
				assert_ok!(margin_withdraw(&ALICE::get(), dollar(4700)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(9700));
			});
	}

	#[test]
	fn test_margin_trader_take_profit() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(3, 1))]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(4, 1))]));

				assert_ok!(margin_close_position(
					&ALICE::get(),
					0,
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(9700));
				assert_eq!(margin_liquidity(), dollar(5300));
			});
	}

	#[test]
	fn test_margin_trader_stop_lost() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(3, 1))]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(28, 10))]));

				assert_ok!(margin_close_position(
					&ALICE::get(),
					0,
					Price::saturating_from_rational(1, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 + 0.03 = 3.03
				// close_price = 2.8 - 0.03 = 2.77
				// profit = leveraged_held * (close_price - open_price)
				// -1300 = 5000 * (2.77 - 3.03)
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(3700));
				assert_eq!(margin_liquidity(), dollar(11_300));
			});
	}

	#[test]
	fn test_margin_trader_stop_out() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(3, 1))]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));
				assert_ok!(margin_set_risk_threshold(
					EUR_USD,
					Some(risk_threshold(3, 1)),
					Some(risk_threshold(30, 10)),
					Some(risk_threshold(3, 1))
				));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				// equity= FixedI128(10300.000000000000000000), net_position =FixedI128(15000.000000000000000000)
				assert_eq!(
					margin_pool_state(),
					Some(MarginPoolState {
						enp: FixedI128::from_inner(0_686666666666666667),
						ell: FixedI128::from_inner(0_686666666666666667),
						required_deposit: FixedI128::zero()
					})
				);
				// margin = leveraged_amount * price / leverage
				// 1515 = 5000 * 3.03 / 10
				// 2.11827 = 3 * (1 - 1515 * 97% / 5000)
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(22, 10))]));
				assert_noop!(
					margin_trader_margin_call(&ALICE::get()),
					margin_protocol::Error::<Runtime>::SafeTrader
				);
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(21, 10))]));
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(200),
						margin_held: FixedI128::saturating_from_integer(1515),
						margin_level: FixedI128::from_inner(0_013201320132013201),
						free_margin: FixedI128::saturating_from_integer(-1315),
						unrealized_pl: FixedI128::saturating_from_integer(-4800),
					}
				);
				// equity= FixedI128(14800.000000000000000000), net_position =FixedI128(10500.000000000000000000)
				// net_position = held * curr_price_usd = 5000 * 2.1 = 10500
				assert_eq!(
					margin_pool_state(),
					Some(MarginPoolState {
						enp: FixedI128::from_inner(1_409523809523809524),
						ell: FixedI128::from_inner(1_409523809523809524),
						required_deposit: FixedI128::zero()
					})
				);
				assert_ok!(margin_trader_margin_call(&ALICE::get()));
				assert_noop!(
					margin_trader_stop_out(&ALICE::get()),
					margin_protocol::Error::<Runtime>::NotReachedRiskThreshold
				);

				assert_noop!(
					margin_trader_become_safe(&ALICE::get()),
					margin_protocol::Error::<Runtime>::UnsafeTrader
				);
				// Price up become safe
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(22, 10))]));
				assert_ok!(margin_trader_become_safe(&ALICE::get()));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(21, 10))]));
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(200),
						margin_held: FixedI128::saturating_from_integer(1515),
						margin_level: FixedI128::from_inner(0_013201320132013201),
						free_margin: FixedI128::saturating_from_integer(-1315),
						unrealized_pl: FixedI128::saturating_from_integer(-4800),
					}
				);
				assert_eq!(
					margin_pool_state(),
					Some(MarginPoolState {
						enp: FixedI128::from_inner(1_409523809523809524),
						ell: FixedI128::from_inner(1_409523809523809524),
						required_deposit: FixedI128::zero()
					})
				);
				assert_ok!(margin_trader_margin_call(&ALICE::get()));

				// Deposit become safe
				assert_ok!(margin_deposit(&ALICE::get(), dollar(500)));
				assert_ok!(margin_trader_become_safe(&ALICE::get()));

				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(19, 10))]));
				assert_ok!(margin_trader_stop_out(&ALICE::get()));

				assert_eq!(collateral_balance(&ALICE::get()), dollar(4500));
				assert_eq!(margin_balance(&ALICE::get()), FixedI128::saturating_from_integer(0));
				assert_eq!(margin_liquidity(), dollar(15_500));
			});
	}

	#[test]
	fn test_margin_liquidity_pool_force_close() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(20_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(3, 1))]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));
				assert_ok!(margin_set_risk_threshold(
					EUR_USD,
					Some(risk_threshold(3, 1)),
					Some(risk_threshold(30, 10)),
					Some(risk_threshold(3, 1))
				));

				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				// 4.4 = 3 * (1 + 10000 * 70% / 3.01 / 5000)
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(38, 10))]));
				assert_noop!(
					margin_liquidity_pool_margin_call(),
					margin_protocol::Error::<Runtime>::SafePool
				);
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(42, 10))]));
				assert_ok!(margin_liquidity_pool_margin_call());
				assert_noop!(
					margin_liquidity_pool_force_close(),
					margin_protocol::Error::<Runtime>::NotReachedRiskThreshold
				);

				assert_noop!(
					margin_liquidity_pool_become_safe(),
					margin_protocol::Error::<Runtime>::UnsafePool
				);
				// Price up become safe
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(38, 10))]));
				assert_ok!(margin_liquidity_pool_become_safe());
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(42, 10))]));
				assert_ok!(margin_liquidity_pool_margin_call());

				// Deposit become safe
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(2200)));
				assert_ok!(margin_liquidity_pool_become_safe());

				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(50, 10))]));
				assert_eq!(collateral_balance(&TreasuryAccount::get()), 0);
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_ok!(margin_liquidity_pool_force_close());

				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(14700));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_liquidity(), 2200_000000000000000000);
				assert_eq!(collateral_balance(&TreasuryAccount::get()), 300_000000000000000000);
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
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(20_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(9000)));
				assert_ok!(margin_deposit(&BOB::get(), dollar(9000)));
				assert_eq!(margin_liquidity(), dollar(20_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(9000));
				assert_eq!(margin_balance(&BOB::get()), fixed_i128_dollar(9000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(1000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(1000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(3, 1))]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				// ALICE open position
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(9000));

				// BOB open position
				assert_ok!(margin_open_position(
					&BOB::get(),
					EUR_USD,
					ShortTen,
					dollar(6000),
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), fixed_i128_dollar(9000));

				// ALICE open position and BOB close position
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(31, 10))]));
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTwenty,
					dollar(1000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(9000));
				assert_ok!(margin_close_position(
					&BOB::get(),
					1,
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), fixed_i128_dollar(8040));

				// ALICE close position and BOB open position
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(29, 10))]));
				assert_ok!(margin_close_position(
					&ALICE::get(),
					0,
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(8200));
				assert_ok!(margin_open_position(
					&BOB::get(),
					EUR_USD,
					ShortTwenty,
					dollar(2000),
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), fixed_i128_dollar(8040));

				// close all
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(28, 10))]));
				assert_ok!(margin_close_position(
					&ALICE::get(),
					2,
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(7840));
				assert_ok!(margin_close_position(
					&BOB::get(),
					3,
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), fixed_i128_dollar(8120));
				assert_eq!(margin_liquidity(), dollar(22_040));
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
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(20_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(20_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(9000)));
				assert_ok!(margin_deposit(&BOB::get(), dollar(9000)));
				assert_eq!(margin_liquidity(), dollar(20_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(9000));
				assert_eq!(margin_balance(&BOB::get()), fixed_i128_dollar(9000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(1000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(1000));
				assert_ok!(set_oracle_price(vec![
					(FEUR, Price::saturating_from_rational(3, 1)),
					(FJPY, Price::saturating_from_rational(5, 1))
				]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));
				assert_ok!(margin_set_spread(JPY_EUR, Price::from_fraction(0.03)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_accumulate(JPY_EUR, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));
				assert_ok!(margin_set_mock_swap_rate(JPY_EUR));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_enable_trading_pair(JPY_EUR));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(JPY_EUR));

				assert_eq!(
					margin_pool_state(),
					Some(MarginPoolState {
						enp: FixedI128::max_value(),
						ell: FixedI128::max_value(),
						required_deposit: FixedI128::zero()
					})
				);
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(9000),
						margin_held: FixedI128::zero(),
						margin_level: FixedI128::max_value(),
						free_margin: FixedI128::saturating_from_integer(9000),
						unrealized_pl: FixedI128::zero()
					}
				);
				assert_eq!(
					margin_trader_state(&BOB::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(9000),
						margin_held: FixedI128::zero(),
						margin_level: FixedI128::max_value(),
						free_margin: FixedI128::saturating_from_integer(9000),
						unrealized_pl: FixedI128::zero()
					}
				);
				// ALICE open position
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(9000));
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(8700),
						margin_held: FixedI128::saturating_from_integer(1515),
						margin_level: FixedI128::from_inner(574257425742574257),
						free_margin: FixedI128::saturating_from_integer(7185),
						unrealized_pl: FixedI128::saturating_from_integer(-300)
					}
				);

				// BOB open position
				assert_ok!(margin_open_position(
					&BOB::get(),
					JPY_EUR,
					ShortTen,
					dollar(6000),
					Price::saturating_from_rational(1, 1)
				));
				assert_eq!(margin_balance(&BOB::get()), fixed_i128_dollar(9000));
				assert_eq!(
					margin_trader_state(&BOB::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(7920),
						margin_held: FixedI128::from_inner(2945999999999999998800),
						margin_level: FixedI128::from_inner(268839103869653768),
						free_margin: FixedI128::from_inner(4974000000000000001200),
						unrealized_pl: FixedI128::saturating_from_integer(-1080)
					}
				);

				// ALICE open position and BOB close position
				assert_ok!(set_oracle_price(vec![
					(FEUR, Price::saturating_from_rational(31, 10)),
					(FJPY, Price::saturating_from_rational(49, 10))
				]));
				assert_ok!(margin_open_position(
					&ALICE::get(),
					JPY_EUR,
					LongTwenty,
					dollar(1000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(9000));
				assert_ok!(margin_close_position(
					&BOB::get(),
					1,
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(
					margin_balance(&BOB::get()),
					FixedI128::from_inner(9483999999999999999600)
				);
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::saturating_from_integer(9014),
						margin_held: FixedI128::from_inner(1764649999999999999900),
						margin_level: FixedI128::from_inner(447500372337784838),
						free_margin: FixedI128::from_inner(7249350000000000000100),
						unrealized_pl: FixedI128::saturating_from_integer(14)
					}
				);
				assert_eq!(
					margin_trader_state(&BOB::get()),
					MarginTraderState {
						equity: FixedI128::from_inner(9483999999999999999600),
						margin_held: FixedI128::zero(),
						margin_level: FixedI128::max_value(),
						free_margin: FixedI128::from_inner(9483999999999999999600),
						unrealized_pl: FixedI128::zero()
					}
				);

				// ALICE close position and BOB open position
				assert_ok!(set_oracle_price(vec![
					(FEUR, Price::saturating_from_rational(29, 10)),
					(FJPY, Price::saturating_from_rational(51, 10))
				]));
				assert_ok!(margin_close_position(
					&ALICE::get(),
					0,
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(8200));
				assert_ok!(margin_open_position(
					&BOB::get(),
					EUR_USD,
					ShortTwenty,
					dollar(2000),
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(
					margin_balance(&BOB::get()),
					FixedI128::from_inner(9483999999999999999600)
				);
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::from_inner(8542_129032258064515700),
						margin_held: FixedI128::from_inner(249_649999999999999900),
						margin_level: FixedI128::from_inner(1_828808607913147372),
						free_margin: FixedI128::from_inner(8292_479032258064515800),
						unrealized_pl: FixedI128::from_inner(342_129032258064515700)
					}
				);
				assert_eq!(
					margin_trader_state(&BOB::get()),
					MarginTraderState {
						equity: FixedI128::from_inner(9363999999999999999600),
						margin_held: FixedI128::saturating_from_integer(287),
						margin_level: FixedI128::from_inner(1631358885017421603),
						free_margin: FixedI128::from_inner(9076999999999999999600),
						unrealized_pl: FixedI128::saturating_from_integer(-120)
					}
				);

				// close all
				assert_ok!(set_oracle_price(vec![
					(FEUR, Price::saturating_from_rational(28, 10)),
					(FJPY, Price::saturating_from_rational(52, 10))
				]));
				assert_ok!(margin_close_position(
					&ALICE::get(),
					2,
					Price::saturating_from_rational(1, 1)
				));
				assert_eq!(
					margin_balance(&ALICE::get()),
					FixedI128::from_inner(8806193548387096773600)
				);
				assert_ok!(margin_close_position(
					&BOB::get(),
					3,
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(
					margin_balance(&BOB::get()),
					FixedI128::from_inner(9563999999999999999600)
				);
				assert_eq!(margin_liquidity(), 19629806451612903226800);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(1000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(1000));
				assert_eq!(
					margin_pool_state(),
					Some(MarginPoolState {
						enp: FixedI128::max_value(),
						ell: FixedI128::max_value(),
						required_deposit: FixedI128::zero()
					})
				);
				assert_eq!(
					margin_trader_state(&ALICE::get()),
					MarginTraderState {
						equity: FixedI128::from_inner(8806193548387096773600),
						margin_held: FixedI128::zero(),
						margin_level: FixedI128::max_value(),
						free_margin: FixedI128::from_inner(8806193548387096773600),
						unrealized_pl: FixedI128::zero()
					}
				);
				assert_eq!(
					margin_trader_state(&BOB::get()),
					MarginTraderState {
						equity: FixedI128::from_inner(9563999999999999999600),
						margin_held: FixedI128::zero(),
						margin_level: FixedI128::max_value(),
						free_margin: FixedI128::from_inner(9563999999999999999600),
						unrealized_pl: FixedI128::zero()
					}
				);
			});
	}

	#[test]
	fn test_margin_accumulate_swap() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(3, 1))]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				// LongTen
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));

				margin_execute_time(1 * ONE_MINUTE..9 * ONE_MINUTE);

				assert_ok!(margin_close_position(
					&ALICE::get(),
					0,
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 + 0.03 = 3.03
				// close_price = 3 - 0.03 = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				// accumulated_swap_usd_value
				//   = leveraged_debits * (accumulated_swap_rate - open_accumulated_swap_rate))
				// -151.5 = 5000 * 3.03 * (-0.01 - 0)
				assert_eq!(
					margin_balance(&ALICE::get()),
					FixedI128::from_inner(4548500000000000000000)
				);
				assert_eq!(margin_liquidity(), 10451500000000000000000);

				// ShortTen
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					ShortTen,
					dollar(5000),
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(
					margin_balance(&ALICE::get()),
					FixedI128::from_inner(4548_500000000000000000)
				);

				margin_execute_time(9 * ONE_MINUTE..22 * ONE_MINUTE);

				assert_ok!(margin_close_position(
					&ALICE::get(),
					1,
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 + 0.03 = 3.03
				// close_price = 3 - 0.03 = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				// accumulated_swap_usd_value =
				//   leveraged_debits * (accumulated_swap_rate - open_accumulated_swap_rate)
				// 304.515 = 5000 * 2.97 * (0.03 - 0.01)
				assert_eq!(
					margin_balance(&ALICE::get()),
					FixedI128::from_inner(4545_500000000000000000)
				);
				assert_eq!(margin_liquidity(), 10454_500000000000000000);
				assert_ok!(margin_withdraw(&ALICE::get(), 4545_500000000000000000));
				assert_eq!(collateral_balance(&ALICE::get()), 9545_500000000000000000);
			});
	}

	#[test]
	fn test_margin_accumulate_swap_with_additional_swap() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(10_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(margin_create_pool());
				assert_eq!(collateral_balance(&POOL::get()), dollar(10_000));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(10_000));
				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(5000)));
				assert_eq!(margin_liquidity(), dollar(10_000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![(FEUR, Price::saturating_from_rational(3, 1))]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Price::from_fraction(0.03)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10 * ONE_MINUTE, 1 * ONE_MINUTE));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				// set_additional_swap_rate, so long = -0.0101, short = 0.0099
				assert_ok!(margin_set_additional_swap(one_percent()));
				println!(
					"long_rate = {:?}, short_rate = {:?}",
					MarginLiquidityPools::swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, true),
					MarginLiquidityPools::swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, false)
				);
				// LongTen
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					LongTen,
					dollar(5000),
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), fixed_i128_dollar(5000));

				margin_execute_time(1 * ONE_MINUTE..9 * ONE_MINUTE);

				assert_ok!(margin_close_position(
					&ALICE::get(),
					0,
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 + 0.03 = 3.03
				// close_price = 3 - 0.03 = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				// accumulated_swap_usd_value =
				//   leveraged_debits * (accumulated_swap_rate - open_accumulated_swap_rate)
				// -153.015 = 5000 * 3.03 * (-0.0101 - 0)
				assert_eq!(
					margin_balance(&ALICE::get()),
					FixedI128::from_inner(4546_985000000000000000)
				);
				assert_eq!(margin_liquidity(), 10453_015000000000000000);

				// ShortTen
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					ShortTen,
					dollar(5000),
					Price::saturating_from_rational(2, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(
					margin_balance(&ALICE::get()),
					FixedI128::from_inner(4546_985000000000000000)
				);

				margin_execute_time(9 * ONE_MINUTE..22 * ONE_MINUTE);

				assert_ok!(margin_close_position(
					&ALICE::get(),
					1,
					Price::saturating_from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 + 0.03 = 3.03
				// close_price = 3 - 0.03 = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				// accumulated_swap_usd_value =
				//   leveraged_debits * (accumulated_swap_rate - open_accumulated_swap_rate)
				// 294.03 = 5000 * 2.97 * (0.0297 - 0.0099)
				assert_eq!(
					margin_balance(&ALICE::get()),
					FixedI128::from_inner(4541_015000000000000000)
				);
				assert_eq!(margin_liquidity(), 10458_985000000000000000);
			});
	}

	#[test]
	fn test_margin_identity() {
		ExtBuilder::default().build().execute_with(|| {
			assert_ok!(margin_create_pool());
			assert_eq!(native_currency_balance(&POOL::get()), 100_000 * DOLLARS);

			// set identity
			assert_ok!(margin_set_identity());
			assert_eq!(native_currency_balance(&POOL::get()), 90_000 * DOLLARS);

			// modify identity
			assert_ok!(margin_set_identity());
			assert_eq!(native_currency_balance(&POOL::get()), 90_000 * DOLLARS);
			assert_ok!(margin_verify_identity());
			assert_eq!(native_currency_balance(&POOL::get()), 90_000 * DOLLARS);

			// clear identity
			assert_ok!(margin_clear_identity());
			assert_eq!(native_currency_balance(&POOL::get()), 100_000 * DOLLARS);

			assert_ok!(margin_set_identity());
			assert_eq!(native_currency_balance(&POOL::get()), 90_000 * DOLLARS);
			// synthetic
			assert_ok!(synthetic_create_pool());
			assert_ok!(synthetic_set_identity());
			assert_eq!(native_currency_balance(&POOL::get()), 80_000 * DOLLARS);

			// remove identity
			assert_ok!(margin_remove_pool(&POOL::get()));
			assert_eq!(native_currency_balance(&POOL::get()), 90_000 * DOLLARS);
			assert_ok!(synthetic_remove_pool(&POOL::get()));
			assert_eq!(native_currency_balance(&POOL::get()), 100_000 * DOLLARS);
		});
	}

	#[test]
	fn test_margin_transfer_liquidity_pool() {
		ExtBuilder::default()
			.balances(vec![
				(POOL::get(), AUSD, dollar(20_000)),
				(ALICE::get(), AUSD, dollar(10_000)),
			])
			.build()
			.execute_with(|| {
				assert_ok!(margin_create_pool());
				assert_eq!(native_currency_balance(&POOL::get()), 100_000 * DOLLARS);

				assert_ok!(margin_deposit_liquidity(&POOL::get(), dollar(10_000)));
				assert_ok!(margin_deposit_liquidity(&ALICE::get(), dollar(5000)));
				assert_ok!(margin_deposit(&ALICE::get(), dollar(4000)));
				assert_eq!(margin_liquidity(), dollar(15_000));

				// set identity
				assert_ok!(margin_set_identity());
				assert_eq!(native_currency_balance(&POOL::get()), 90_000 * DOLLARS);

				// transfer liquidity pool to ALICE
				assert_ok!(margin_transfer_liquidity_pool(
					&POOL::get(),
					LIQUIDITY_POOL_ID_0,
					ALICE::get()
				));
				assert_eq!(native_currency_balance(&POOL::get()), 100_000 * DOLLARS);

				assert_noop!(
					margin_withdraw_liquidity(&POOL::get(), dollar(1000)),
					base_liquidity_pools::Error::<Runtime, BaseLiquidityPoolsMarginInstance>::NoPermission
				);
				assert_ok!(margin_withdraw_liquidity(&ALICE::get(), dollar(1000)));
				assert_ok!(margin_withdraw(&ALICE::get(), dollar(1000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(3_000));
				assert_eq!(margin_liquidity(), dollar(14_000));

				// transfer liquidity pool to BOB
				assert_ok!(margin_transfer_liquidity_pool(
					&ALICE::get(),
					LIQUIDITY_POOL_ID_0,
					BOB::get()
				));
				assert_eq!(collateral_balance(&BOB::get()), 0);
				assert_ok!(margin_withdraw_liquidity(&BOB::get(), dollar(1000)));
				assert_ok!(margin_withdraw(&ALICE::get(), dollar(1000)));
				assert_eq!(margin_liquidity(), dollar(13_000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(1000));

				// remove pool
				assert_ok!(margin_remove_pool(&BOB::get()));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(4_000));
				assert_eq!(collateral_balance(&BOB::get()), dollar(14_000));
				assert_ok!(margin_withdraw(&ALICE::get(), dollar(1000)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5_000));
			});
	}
}
