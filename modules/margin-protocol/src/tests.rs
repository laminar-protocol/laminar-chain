//! Unit tests for the margin protocol module.

#![cfg(test)]

use super::*;
use mock::*;

use core::num::NonZeroI128;
use frame_support::{assert_noop, assert_ok};
use primitives::Leverage;
use sp_runtime::PerThing;

const EUR_JPY_PAIR: TradingPair = TradingPair {
	base: CurrencyId::FJPY,
	quote: CurrencyId::FEUR,
};

// `n` is a natural currency amount by cent, with 2 fractional digits precision
fn fixed128_from_natural_currency_cent(n: i128) -> Fixed128 {
	Fixed128::from_parts(n * 1_000_000_000_000_000_0)
}

// `b` is a natural currency amount by cent, with 2 fractional digits precision
fn balance_from_natural_currency_cent(b: u128) -> Balance {
	b * 1_000_000_000_000_000_0
}

fn eur_jpy_long() -> Position<Runtime> {
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_JPY_PAIR,
		leverage: Leverage::LongTwenty,
		leveraged_held: Fixed128::from_natural(100_000),
		leveraged_debits: Fixed128::from_natural(-14_104_090),
		leveraged_held_in_usd: fixed128_from_natural_currency_cent(-131_813_93),
		open_accumulated_swap_rate: Fixed128::from_natural(1),
		open_margin: balance_from_natural_currency_cent(6_591),
	}
}

fn eur_jpy_short() -> Position<Runtime> {
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_JPY_PAIR,
		leverage: Leverage::ShortTwenty,
		leveraged_held: Fixed128::from_natural(-100_000),
		leveraged_debits: Fixed128::from_natural(14_175_810),
		leveraged_held_in_usd: fixed128_from_natural_currency_cent(133_734_06),
		open_accumulated_swap_rate: Fixed128::from_natural(1),
		open_margin: balance_from_natural_currency_cent(6_687),
	}
}

#[test]
fn unrealized_pl_of_long_position_works() {
	ExtBuilder::default()
		// USD/JPY = 110
		.price(CurrencyId::FJPY, (1, 110))
		// EUR/JPY = 140 => EUR/USD = 140/110
		.price(CurrencyId::FEUR, (140, 110))
		.build()
		.execute_with(|| {
			assert_eq!(
				MarginProtocol::_unrealized_pl_of_position(&eur_jpy_long()),
				Ok(Fixed128::from_parts(-1073545454545441750827)),
			);
		});
}

#[test]
fn unrealized_pl_of_short_position_works() {
	ExtBuilder::default()
		// USD/JPY = 110
		.price(CurrencyId::FJPY, (1, 110))
		// EUR/JPY = 140 => EUR/USD = 140/110
		.price(CurrencyId::FEUR, (140, 110))
		.build()
		.execute_with(|| {
			assert_eq!(
				MarginProtocol::_unrealized_pl_of_position(&eur_jpy_short()),
				Ok(Fixed128::from_parts(1470999999999987141081)),
			);
		});
}

#[test]
fn unrealized_pl_of_trader_sums_all_positions() {
	ExtBuilder::default()
		// USD/JPY = 110
		.price(CurrencyId::FJPY, (1, 110))
		// EUR/JPY = 140 => EUR/USD = 140/110
		.price(CurrencyId::FEUR, (140, 110))
		.build()
		.execute_with(|| {
			<Positions<Runtime>>::insert(0, eur_jpy_long());
			<Positions<Runtime>>::insert(1, eur_jpy_short());
			<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0, 1]);
			assert_eq!(
				MarginProtocol::_unrealized_pl_of_trader(&ALICE),
				Ok(Fixed128::from_parts(397454545454545390254))
			);
		});
}

#[test]
fn margin_held_sums_all_open_margin() {
	ExtBuilder::default().build().execute_with(|| {
		<Positions<Runtime>>::insert(0, eur_jpy_long());
		<Positions<Runtime>>::insert(1, eur_jpy_short());
		<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0, 1]);
		assert_eq!(
			MarginProtocol::_margin_held(&ALICE),
			balance_from_natural_currency_cent(13_278)
		);
	});
}

#[test]
fn free_balance_equal_to_balance_sub_margin_held() {
	ExtBuilder::default().build().execute_with(|| {
		<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(15_000));
		<Positions<Runtime>>::insert(0, eur_jpy_long());
		<Positions<Runtime>>::insert(1, eur_jpy_short());
		<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0, 1]);
		assert_eq!(
			MarginProtocol::_free_balance(&ALICE),
			balance_from_natural_currency_cent(1_722)
		);
	});
}

#[test]
fn free_balance_is_zero_if_margin_held_bigger_than_balance() {
	ExtBuilder::default().build().execute_with(|| {
		<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(10_000));
		<Positions<Runtime>>::insert(0, eur_jpy_long());
		<Positions<Runtime>>::insert(1, eur_jpy_short());
		<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0, 1]);
		assert_eq!(MarginProtocol::_free_balance(&ALICE), 0,);
	});
}

const EUR_USD_PAIR: TradingPair = TradingPair {
	base: CurrencyId::AUSD,
	quote: CurrencyId::FEUR,
};

fn eur_usd_long_1() -> Position<Runtime> {
	let open_rate =
		Fixed128::from_natural(1).saturating_add(Fixed128::from_rational(36_87, NonZeroI128::new(100_000_00).unwrap()));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::LongFive,
		leveraged_held: Fixed128::from_natural(100_000),
		leveraged_debits: fixed128_from_natural_currency_cent(-120_420_30),
		leveraged_held_in_usd: fixed128_from_natural_currency_cent(-120_420_30),
		open_accumulated_swap_rate: open_rate,
		open_margin: balance_from_natural_currency_cent(24_084),
	}
}

fn eur_usd_long_2() -> Position<Runtime> {
	let open_rate =
		Fixed128::from_natural(1).saturating_add(Fixed128::from_rational(18_43, NonZeroI128::new(100_000_00).unwrap()));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::LongTwenty,
		leveraged_held: Fixed128::from_natural(100_000),
		leveraged_debits: fixed128_from_natural_currency_cent(-119_419_30),
		leveraged_held_in_usd: fixed128_from_natural_currency_cent(-119_419_30),
		open_accumulated_swap_rate: open_rate,
		open_margin: balance_from_natural_currency_cent(5_971),
	}
}

fn eur_usd_short_1() -> Position<Runtime> {
	let open_rate =
		Fixed128::from_natural(1).saturating_sub(Fixed128::from_rational(10_96, NonZeroI128::new(100_000_00).unwrap()));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::ShortTen,
		leveraged_held: Fixed128::from_natural(-100_000),
		leveraged_debits: fixed128_from_natural_currency_cent(119_780_10),
		leveraged_held_in_usd: fixed128_from_natural_currency_cent(119_780_10),
		open_accumulated_swap_rate: open_rate,
		open_margin: balance_from_natural_currency_cent(11_978),
	}
}

fn eur_usd_short_2() -> Position<Runtime> {
	let open_rate =
		Fixed128::from_natural(1).saturating_sub(Fixed128::from_rational(3_65, NonZeroI128::new(100_000_00).unwrap()));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::ShortFifty,
		leveraged_held: Fixed128::from_natural(-200_000),
		leveraged_debits: fixed128_from_natural_currency_cent(237_362_40),
		leveraged_held_in_usd: fixed128_from_natural_currency_cent(237_362_40),
		open_accumulated_swap_rate: open_rate,
		open_margin: balance_from_natural_currency_cent(4_747),
	}
}

#[test]
fn accumulated_swap_rate_of_long_position_works() {
	ExtBuilder::default()
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.build()
		.execute_with(|| {
			assert_eq!(
				MarginProtocol::_accumulated_swap_rate_of_position(&eur_usd_long_1()),
				Ok(fixed128_from_natural_currency_cent(-36_87))
			);
		});
}

#[test]
fn accumulated_swap_rate_of_short_position_works() {
	ExtBuilder::default()
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.build()
		.execute_with(|| {
			assert_eq!(
				MarginProtocol::_accumulated_swap_rate_of_position(&eur_usd_short_1()),
				Ok(fixed128_from_natural_currency_cent(10_96))
			);
		});
}

#[test]
fn accumulated_swap_rate_of_trader_sums_all_positions() {
	ExtBuilder::default()
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.build()
		.execute_with(|| {
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_short_1());
			<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0, 1]);
			assert_eq!(
				MarginProtocol::_accumulated_swap_rate_of_trader(&ALICE),
				Ok(fixed128_from_natural_currency_cent(-25_91))
			);
		});
}

#[test]
fn equity_of_trader_works() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(120_000_00));
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0, 1, 2, 3]);
			assert_eq!(
				MarginProtocol::_equity_of_trader(&ALICE),
				Ok(fixed128_from_natural_currency_cent(116_665_86))
			);
		});
}

#[test]
fn margin_level_without_new_position_works() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(120_000_00));
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0, 1, 2, 3]);
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				// 19.44%
				Ok(Fixed128::from_parts(195426060513372176))
			);
		});
}

#[test]
fn margin_level_with_new_position_works() {
	ExtBuilder::default().build().execute_with(|| {
		<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(120_000_00));
		assert_eq!(
			MarginProtocol::_margin_level(&ALICE, Some(eur_usd_long_1())),
			// 120_000_00 / 120_420_30 = 0.996509724689275811
			Ok(Fixed128::from_parts(996509724689275811))
		);
	});
}

#[test]
fn margin_level_without_any_opened_position_is_max() {
	ExtBuilder::default().build().execute_with(|| {
		<Balances<Runtime>>::insert(ALICE, 1);
		assert_eq!(MarginProtocol::_margin_level(&ALICE, None), Ok(Fixed128::max_value()));
	});
}

#[test]
fn ensure_trader_safe_works() {
	let margin_call_100_percent = RiskThreshold {
		margin_call: Permill::one(),
		stop_out: Permill::zero(),
	};
	let margin_call_99_percent = RiskThreshold {
		margin_call: Permill::from_percent(99),
		stop_out: Permill::zero(),
	};

	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(margin_call_100_percent)
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_held_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				open_margin: balance_from_natural_currency_cent(100),
			};
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, Some(position.clone())),
				Ok(Fixed128::from_natural(1))
			);

			// 100% == 100%, unsafe
			assert_noop!(
				MarginProtocol::_ensure_trader_safe(&ALICE, Some(position.clone())),
				Error::<Runtime>::TraderWouldBeUnsafe
			);
			// 100% > 99%, safe
			TraderRiskThreshold::put(margin_call_99_percent);
			assert_ok!(MarginProtocol::_ensure_trader_safe(&ALICE, Some(position.clone())));

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0]);
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_natural(1))
			);

			TraderRiskThreshold::put(margin_call_100_percent);

			// 100% == 100%, unsafe
			assert_noop!(
				MarginProtocol::_ensure_trader_safe(&ALICE, None),
				Error::<Runtime>::UnsafeTrader
			);
			// 100% > 99%, safe
			TraderRiskThreshold::put(margin_call_99_percent);
			assert_ok!(MarginProtocol::_ensure_trader_safe(&ALICE, None));
		});
}

#[test]
fn trader_margin_call_should_work() {
	let risk_threshold = RiskThreshold {
		margin_call: Permill::from_percent(5),
		stop_out: Permill::from_percent(3),
	};

	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold)
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_held_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				open_margin: balance_from_natural_currency_cent(100),
			};

			assert_eq!(MarginProtocol::_margin_level(&ALICE, None), Ok(Fixed128::max_value()));

			// without position
			assert_noop!(
				MarginProtocol::trader_margin_call(Origin::ROOT, ALICE),
				Error::<Runtime>::CannotTraderMarginCall
			);

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0]);
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_natural(1))
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(1, 20)));
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_rational(5, NonZeroI128::new(100).unwrap()))
			);

			assert_ok!(MarginProtocol::trader_margin_call(Origin::ROOT, ALICE));
		});
}

#[test]
fn trader_become_safe_should_work() {
	let risk_threshold = RiskThreshold {
		margin_call: Permill::from_percent(5),
		stop_out: Permill::from_percent(3),
	};

	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold)
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_held_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				open_margin: balance_from_natural_currency_cent(100),
			};

			// without position
			assert_ok!(MarginProtocol::trader_become_safe(Origin::ROOT, ALICE));

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0]);
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_natural(1))
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(4, 100)));
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_rational(4, NonZeroI128::new(100).unwrap()))
			);
			assert_ok!(MarginProtocol::trader_margin_call(Origin::ROOT, ALICE));
			assert_noop!(
				MarginProtocol::trader_become_safe(Origin::ROOT, ALICE),
				Error::<Runtime>::CannotTraderBecomeSafe
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(5, 100)));
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_rational(5, NonZeroI128::new(100).unwrap()))
			);
			assert_noop!(
				MarginProtocol::trader_become_safe(Origin::ROOT, ALICE),
				Error::<Runtime>::CannotTraderBecomeSafe
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(6, 100)));
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_rational(6, NonZeroI128::new(100).unwrap()))
			);
			assert_ok!(MarginProtocol::trader_become_safe(Origin::ROOT, ALICE));
		});
}
#[test]
fn trader_liquidate_should_work() {
	let risk_threshold = RiskThreshold {
		margin_call: Permill::from_percent(5),
		stop_out: Permill::from_percent(3),
	};

	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold)
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, balance_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_held_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				open_margin: balance_from_natural_currency_cent(100),
			};

			// without position
			assert_noop!(
				MarginProtocol::trader_liquidate(Origin::ROOT, ALICE),
				Error::<Runtime>::CannotTraderLiquidate
			);

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, MOCK_POOL, vec![0]);
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_natural(1))
			);

			// trader_liquidate without trader_margin_call
			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(3, 100)));
			assert_eq!(
				MarginProtocol::_margin_level(&ALICE, None),
				Ok(Fixed128::from_rational(3, NonZeroI128::new(100).unwrap()))
			);
			assert_eq!(
				MarginProtocol::_free_balance(&ALICE),
				balance_from_natural_currency_cent(0)
			);
			// TODO: need implementation close_positions
			//assert_ok!(MarginProtocol::trader_liquidate(Origin::ROOT, ALICE));
			//assert_eq!(
			//	MarginProtocol::_free_balance(&ALICE),
			//	balance_from_natural_currency_cent(0)
			//);
		});
}
