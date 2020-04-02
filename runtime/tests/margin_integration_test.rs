/// tests for this module

#[cfg(test)]

mod tests {
	use frame_support::{assert_noop, assert_ok};
	use laminar_runtime::{
		tests::*,
		CurrencyId::{AUSD, FEUR, FJPY},
		MaxSwap, MockLaminarTreasury, Runtime,
	};

	use module_primitives::Leverage::*;
	use module_traits::{MarginProtocolLiquidityPools, Treasury};
	use orml_prices::Price;
	use sp_runtime::{traits::OnInitialize, Permill};

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

				assert_ok!(margin_set_enabled_trades());

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
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
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
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
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
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
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
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
				assert_ok!(margin_create_pool());
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
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
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
					Price::from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				// margin = leveraged_amount * price / leverage
				// 1505 = 5000 * 3.01 / 10
				// 2.12409 = 3 * (1 - 1505 * 97% / 5000)
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(22, 10))]));
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
				assert_ok!(margin_create_pool());
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
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
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
					Price::from_rational(4, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				// 4.4 = 3 * (1 + 10000 * 70% / 3.01 / 5000)
				assert_ok!(set_oracle_price(vec![(FEUR, Price::from_rational(41, 10))]));
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
				assert_ok!(margin_create_pool());
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
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
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
				assert_ok!(margin_create_pool());
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
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));
				assert_ok!(margin_set_spread(JPY_EUR, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_accumulate(JPY_EUR, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));
				assert_ok!(margin_set_mock_swap_rate(JPY_EUR));

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
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
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
				// accumulated_swap = leveraged_held * (accumulated_swap_rate - open_accumulated_swap_rate)
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

				for i in 9..22 {
					MarginLiquidityPools::on_initialize(i);
					println!(
						"accumulated_long_rate = {:?}, accumulated_short_rate = {:?}",
						MarginLiquidityPools::get_accumulated_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, true),
						MarginLiquidityPools::get_accumulated_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, false)
					);
				}

				assert_ok!(margin_close_position(&ALICE::get(), 1, Price::from_rational(4, 1)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 3 * (1 - 0.01) = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				// accumulated_swap = leveraged_held * (accumulated_swap_rate - open_accumulated_swap_rate)
				// 101.505 = 5000 * (0.030301 - 0.01)
				assert_eq!(margin_balance(&ALICE::get()), 4248495000000000000000);
				assert_eq!(margin_liquidity(), 10751505000000000000000);
				assert_ok!(margin_withdraw(&ALICE::get(), 4248495000000000000000));
				assert_eq!(collateral_balance(&ALICE::get()), 9248495000000000000000);
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
				assert_eq!(margin_balance(&ALICE::get()), dollar(5000));
				assert_eq!(collateral_balance(&POOL::get()), 0);
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_ok!(set_oracle_price(vec![
					(AUSD, Price::from_rational(1, 1)),
					(FEUR, Price::from_rational(3, 1))
				]));
				assert_ok!(margin_set_enabled_trades());
				assert_ok!(margin_set_spread(EUR_USD, Permill::from_percent(1)));

				assert_ok!(margin_set_accumulate(EUR_USD, 10, 1));
				assert_ok!(margin_set_min_leveraged_amount(dollar(100)));
				assert_ok!(margin_set_default_min_leveraged_amount(dollar(1000)));
				assert_ok!(margin_set_mock_swap_rate(EUR_USD));

				assert_ok!(margin_enable_trading_pair(EUR_USD));
				assert_ok!(margin_liquidity_pool_enable_trading_pair(EUR_USD));

				// set_additional_swap
				assert_ok!(margin_set_additional_swap(one_percent()));
				println!(
					"long_rate = {:?}, short_rate = {:?}",
					MarginLiquidityPools::get_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, true),
					MarginLiquidityPools::get_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, false)
				);
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
				// accumulated_swap = leveraged_held * (accumulated_swap_rate - open_accumulated_swap_rate)
				// -100 = 5000 * -0.02
				assert_eq!(margin_balance(&ALICE::get()), dollar(4600));
				assert_eq!(margin_liquidity(), dollar(10400));

				// ShortTen
				assert_ok!(margin_open_position(
					&ALICE::get(),
					EUR_USD,
					ShortTen,
					dollar(5000),
					Price::from_rational(2, 1)
				));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				assert_eq!(margin_balance(&ALICE::get()), dollar(4600));

				for i in 9..22 {
					MarginLiquidityPools::on_initialize(i);
					println!(
						"accumulated_long_rate = {:?}, accumulated_short_rate = {:?}",
						MarginLiquidityPools::get_accumulated_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, true),
						MarginLiquidityPools::get_accumulated_swap_rate(LIQUIDITY_POOL_ID_0, EUR_USD, false)
					);
				}

				assert_ok!(margin_close_position(&ALICE::get(), 1, Price::from_rational(4, 1)));
				assert_eq!(collateral_balance(&ALICE::get()), dollar(5000));
				// open_price = 3 * (1 + 0.01) = 3.03
				// close_price = 3 * (1 - 0.01) = 2.97
				// profit = leveraged_held * (close_price - open_price)
				// -300 = 5000 * (2.97 - 3.03)
				// accumulated_swap = leveraged_held * (accumulated_swap_rate - open_accumulated_swap_rate)
				// 0 = 5000 * 0
				assert_eq!(margin_balance(&ALICE::get()), dollar(4300));
				assert_eq!(margin_liquidity(), dollar(10700));
			});
	}
}
