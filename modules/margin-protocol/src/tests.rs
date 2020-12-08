//! Unit tests for the margin protocol module.

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use primitives::Leverage;
use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainExt, TransactionPoolExt,
};

// `n` is a natural currency amount by cent, with 2 fractional digits precision
fn fixedi128_saturating_from_integer_currency_cent(n: i128) -> FixedI128 {
	FixedI128::from_inner(n * 1_000_000_000_000_000_0)
}

// `b` is a natural currency amount by cent, with 2 fractional digits precision
fn balance_saturating_from_integer_currency_cent(b: u128) -> Balance {
	b * 1_000_000_000_000_000_0
}

fn risk_threshold(margin_call_percent: u32, stop_out_percent: u32) -> RiskThreshold {
	RiskThreshold {
		margin_call: Permill::from_percent(margin_call_percent),
		stop_out: Permill::from_percent(stop_out_percent),
	}
}

fn set_trader_risk_threshold(pair: TradingPair, threshold: RiskThreshold) {
	assert_ok!(MarginProtocol::set_trading_pair_risk_threshold(
		Origin::signed(UpdateOrigin::get()),
		pair,
		Some(threshold),
		None,
		None
	));
}

fn set_enp_risk_threshold(pair: TradingPair, threshold: RiskThreshold) {
	assert_ok!(MarginProtocol::set_trading_pair_risk_threshold(
		Origin::signed(UpdateOrigin::get()),
		pair,
		None,
		Some(threshold),
		None
	));
}

fn set_ell_risk_threshold(pair: TradingPair, threshold: RiskThreshold) {
	assert_ok!(MarginProtocol::set_trading_pair_risk_threshold(
		Origin::signed(UpdateOrigin::get()),
		pair,
		None,
		None,
		Some(threshold)
	));
}

fn positions_snapshot(
	positions_count: u64,
	long_base_amount: FixedI128,
	long_quote_amount: FixedI128,
	short_base_amount: FixedI128,
	short_quote_amount: FixedI128,
) -> PositionsSnapshot {
	PositionsSnapshot {
		positions_count: positions_count,
		long: LeveragedAmounts {
			held: long_base_amount,
			debits: long_quote_amount,
		},
		short: LeveragedAmounts {
			held: short_base_amount,
			debits: short_quote_amount,
		},
	}
}

fn eur_jpy_long() -> Position<Runtime> {
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_JPY_PAIR,
		leverage: Leverage::LongTwenty,
		leveraged_held: FixedI128::saturating_from_integer(100_000),
		leveraged_debits: FixedI128::saturating_from_integer(-14_104_090),
		open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
		margin_held: fixedi128_saturating_from_integer_currency_cent(6_591_00),
	}
}

fn eur_jpy_short() -> Position<Runtime> {
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_JPY_PAIR,
		leverage: Leverage::ShortTwenty,
		leveraged_held: FixedI128::saturating_from_integer(-100_000),
		leveraged_debits: FixedI128::saturating_from_integer(14_175_810),
		open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
		margin_held: fixedi128_saturating_from_integer_currency_cent(6_687_00),
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
				MarginProtocol::unrealized_pl_of_position(&eur_jpy_long()),
				Ok(FixedI128::from_inner(-1073_545454545441749918)),
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
				MarginProtocol::unrealized_pl_of_position(&eur_jpy_short()),
				Ok(FixedI128::from_inner(1470_999999999987141082)),
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
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
			assert_eq!(
				MarginProtocol::unrealized_pl_of_trader(&ALICE, MOCK_POOL),
				Ok(FixedI128::from_inner(397_454545454545391164))
			);
		});
}

#[test]
fn margin_held_sums_all_margin_held() {
	ExtBuilder::default().build().execute_with(|| {
		<Positions<Runtime>>::insert(0, eur_jpy_long());
		<Positions<Runtime>>::insert(1, eur_jpy_short());
		<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
		<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
		assert_eq!(
			MarginProtocol::margin_held(&ALICE, MOCK_POOL),
			fixedi128_saturating_from_integer_currency_cent(13_278_00)
		);
	});
}

fn eur_usd_long_1() -> Position<Runtime> {
	let open_rate =
		FixedI128::saturating_from_integer(1).saturating_add(FixedI128::saturating_from_rational(36_87, 100_000_00));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::LongFive,
		leveraged_held: FixedI128::saturating_from_integer(100_000),
		leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-120_420_30),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixedi128_saturating_from_integer_currency_cent(24_084_00),
	}
}

fn eur_usd_long_2() -> Position<Runtime> {
	let open_rate =
		FixedI128::saturating_from_integer(1).saturating_add(FixedI128::saturating_from_rational(18_43, 100_000_00));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::LongTwenty,
		leveraged_held: FixedI128::saturating_from_integer(100_000),
		leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-119_419_30),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixedi128_saturating_from_integer_currency_cent(5_971_00),
	}
}

fn eur_usd_short_1() -> Position<Runtime> {
	let open_rate =
		FixedI128::saturating_from_integer(1).saturating_add(FixedI128::saturating_from_rational(10_96, 100_000_00));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::ShortTen,
		leveraged_held: FixedI128::saturating_from_integer(-100_000),
		leveraged_debits: fixedi128_saturating_from_integer_currency_cent(119_780_10),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixedi128_saturating_from_integer_currency_cent(11_978_00),
	}
}

fn eur_usd_short_2() -> Position<Runtime> {
	let open_rate =
		FixedI128::saturating_from_integer(1).saturating_add(FixedI128::saturating_from_rational(3_65, 100_000_00));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::ShortFifty,
		leveraged_held: FixedI128::saturating_from_integer(-200_000),
		leveraged_debits: fixedi128_saturating_from_integer_currency_cent(237_362_40),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixedi128_saturating_from_integer_currency_cent(4_747_00),
	}
}

fn jpy_usd_long_1() -> Position<Runtime> {
	let open_rate =
		FixedI128::saturating_from_integer(1).saturating_add(FixedI128::saturating_from_rational(36_87, 100_000_00));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: JPY_USD_PAIR,
		leverage: Leverage::LongFive,
		leveraged_held: FixedI128::saturating_from_integer(100_000),
		leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-120_420_30),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixedi128_saturating_from_integer_currency_cent(24_084_00),
	}
}

#[test]
fn accumulated_swap_rate_of_long_position_works() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (1, 1))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.build()
		.execute_with(|| {
			assert_eq!(
				MarginProtocol::accumulated_swap_rate_of_position(&eur_usd_long_1()),
				Ok(FixedI128::from_inner(-44398964610000000000))
			);
		});
}

#[test]
fn accumulated_swap_rate_of_short_position_works() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (1, 1))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.build()
		.execute_with(|| {
			assert_eq!(
				MarginProtocol::accumulated_swap_rate_of_position(&eur_usd_short_1()),
				Ok(FixedI128::from_inner(-13127898960000000000))
			);
		});
}

#[test]
fn accumulated_swap_rate_of_trader_sums_all_positions() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (1, 1))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.build()
		.execute_with(|| {
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_short_1());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
			assert_eq!(
				MarginProtocol::accumulated_swap_rate_of_trader(&ALICE, MOCK_POOL),
				Ok(FixedI128::from_inner(-57526863570000000000))
			);
		});
}

#[test]
fn equity_of_trader_works() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(120_000_00),
			);
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 2), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 3), ());
			assert_eq!(
				MarginProtocol::equity_of_trader(&ALICE, MOCK_POOL),
				Ok(FixedI128::from_inner(116614700431840000000000))
			);
		});
}

#[test]
fn margin_level_works() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(120_000_00),
			);
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 2), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 3), ());
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				// 19.54%
				Ok(FixedI128::from_inner(195340363524869506))
			);
		});
}

#[test]
fn margin_level_without_any_opened_position_is_max() {
	ExtBuilder::default().build().execute_with(|| {
		<Balances<Runtime>>::insert(ALICE, MOCK_POOL, FixedI128::from_inner(1));
		assert_eq!(
			MarginProtocol::margin_level(&ALICE, MOCK_POOL),
			Ok(FixedI128::max_value())
		);
	});
}

#[test]
fn ensure_trader_safe_works() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};
			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_integer(1))
			);

			// 100% == 100%, unsafe
			assert_noop!(
				MarginProtocol::ensure_trader_safe(&ALICE, MOCK_POOL, Action::None),
				Error::<Runtime>::UnsafeTrader
			);

			// 100% > 99%, safe
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			assert_ok!(MarginProtocol::ensure_trader_safe(&ALICE, MOCK_POOL, Action::None));
		});
}

#[test]
fn equity_of_pool_works() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.build()
		.execute_with(|| {
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 1), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 2), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 3), ());
			let snapshot = positions_snapshot(
				4,
				eur_usd_long_1()
					.leveraged_held
					.checked_add(&eur_usd_long_2().leveraged_held)
					.unwrap(),
				eur_usd_long_1()
					.leveraged_debits
					.checked_add(&eur_usd_long_2().leveraged_debits)
					.unwrap(),
				eur_usd_short_1()
					.leveraged_held
					.checked_add(&eur_usd_short_2().leveraged_held)
					.unwrap(),
				eur_usd_short_1()
					.leveraged_debits
					.checked_add(&eur_usd_short_2().leveraged_debits)
					.unwrap(),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);
			assert_eq!(
				MarginProtocol::equity_of_pool(MOCK_POOL),
				Ok(FixedI128::from_inner(103297_100000000000000000))
			);
		});
}

#[test]
fn enp_and_ell_without_new_position_works() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.build()
		.execute_with(|| {
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 1), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 2), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 3), ());

			let snapshot = positions_snapshot(
				4,
				eur_usd_long_1()
					.leveraged_held
					.checked_add(&eur_usd_long_2().leveraged_held)
					.unwrap(),
				eur_usd_long_1()
					.leveraged_debits
					.checked_add(&eur_usd_long_2().leveraged_debits)
					.unwrap(),
				eur_usd_short_1()
					.leveraged_held
					.checked_add(&eur_usd_short_2().leveraged_held)
					.unwrap(),
				eur_usd_short_1()
					.leveraged_debits
					.checked_add(&eur_usd_short_2().leveraged_debits)
					.unwrap(),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);

			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::None),
				Ok((
					FixedI128::from_inner(0_860809166666666667),
					FixedI128::from_inner(0_286936388888888889)
				))
			);
		});
}

#[test]
fn enp_and_ell_with_new_position_works() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (12, 10))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			// enp = ell = 100_000_00 / 120_000_00
			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::OpenPosition(eur_usd_long_1())),
				Ok((
					FixedI128::from_inner(833333333333333333),
					FixedI128::from_inner(833333333333333333)
				))
			);
		});
}

#[test]
fn enp_and_ell_without_position_with_liquidity_works() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.build()
		.execute_with(|| {
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 1), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 2), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 3), ());
			let snapshot = positions_snapshot(
				4,
				eur_usd_long_1()
					.leveraged_held
					.checked_add(&eur_usd_long_2().leveraged_held)
					.unwrap(),
				eur_usd_long_1()
					.leveraged_debits
					.checked_add(&eur_usd_long_2().leveraged_debits)
					.unwrap(),
				eur_usd_short_1()
					.leveraged_held
					.checked_add(&eur_usd_short_2().leveraged_held)
					.unwrap(),
				eur_usd_short_1()
					.leveraged_debits
					.checked_add(&eur_usd_short_2().leveraged_debits)
					.unwrap(),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);

			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(
					MOCK_POOL,
					Action::Withdraw(balance_saturating_from_integer_currency_cent(10))
				),
				Ok((
					FixedI128::from_inner(0_860808333333333333),
					FixedI128::from_inner(0_286936111111111111)
				))
			);
		});
}

#[test]
fn ensure_liquidity_works() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};

			<Positions<Runtime>>::insert(0, position.clone());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			let snapshot = positions_snapshot(
				1,
				position.leveraged_held,
				position.leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);

			assert_ok!(MarginProtocol::ensure_can_withdraw(MOCK_POOL, 10));

			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));

			assert_noop!(
				MarginProtocol::ensure_can_withdraw(MOCK_POOL, 1),
				Error::<Runtime>::PoolWouldBeUnsafe
			);
		});
}

#[test]
fn ensure_pool_safe_works() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};

			// with new position
			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::OpenPosition(position.clone())),
				Ok((
					FixedI128::saturating_from_integer(1),
					FixedI128::saturating_from_integer(1)
				))
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			assert_ok!(MarginProtocol::ensure_pool_safe(
				MOCK_POOL,
				Action::OpenPosition(position.clone()),
			));

			// ENP 100% == 100%, unsafe
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::ensure_pool_safe(MOCK_POOL, Action::OpenPosition(position.clone())),
				Error::<Runtime>::PoolWouldBeUnsafe
			);

			// ELL 100% == 100%, unsafe
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::ensure_pool_safe(MOCK_POOL, Action::OpenPosition(position.clone())),
				Error::<Runtime>::PoolWouldBeUnsafe
			);

			// without new position
			<Positions<Runtime>>::insert(0, position.clone());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));

			let snapshot = positions_snapshot(
				1,
				position.leveraged_held,
				position.leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);

			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::None),
				Ok((
					FixedI128::saturating_from_integer(1),
					FixedI128::saturating_from_integer(1)
				))
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			assert_ok!(MarginProtocol::ensure_pool_safe(MOCK_POOL, Action::None));

			// ENP 100% == 100%, unsafe
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::ensure_pool_safe(MOCK_POOL, Action::None),
				Error::<Runtime>::UnsafePool
			);

			// ELL 100% == 100%, unsafe
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::ensure_pool_safe(MOCK_POOL, Action::None),
				Error::<Runtime>::UnsafePool
			);
		});
}

#[test]
fn trader_margin_call_should_work() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(5, 3));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};

			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::max_value())
			);

			// without position
			assert_noop!(
				MarginProtocol::trader_margin_call(Origin::none(), ALICE, MOCK_POOL),
				Error::<Runtime>::SafeTrader
			);

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_integer(1))
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(1, 20)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_rational(5, 100))
			);

			assert_ok!(MarginProtocol::trader_margin_call(Origin::none(), ALICE, MOCK_POOL));
		});
}

#[test]
fn trader_become_safe_should_work() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(5, 3));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};

			// without position
			assert_ok!(MarginProtocol::trader_become_safe(Origin::none(), ALICE, MOCK_POOL));

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_integer(1))
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(4, 100)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_rational(4, 100))
			);
			assert_ok!(MarginProtocol::trader_margin_call(Origin::none(), ALICE, MOCK_POOL));
			assert_noop!(
				MarginProtocol::trader_become_safe(Origin::none(), ALICE, MOCK_POOL),
				Error::<Runtime>::UnsafeTrader
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(5, 100)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_rational(5, 100))
			);
			assert_noop!(
				MarginProtocol::trader_become_safe(Origin::none(), ALICE, MOCK_POOL),
				Error::<Runtime>::UnsafeTrader
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(6, 100)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_rational(6, 100))
			);
			assert_ok!(MarginProtocol::trader_become_safe(Origin::none(), ALICE, MOCK_POOL));
		});
}
#[test]
fn trader_stop_out_should_work() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100))
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(5, 3));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};

			// without position
			assert_noop!(
				MarginProtocol::trader_stop_out(Origin::none(), ALICE, MOCK_POOL),
				Error::<Runtime>::NotReachedRiskThreshold
			);

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_integer(1))
			);

			// trader_stop_out without trader_margin_call
			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(3, 100)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE, MOCK_POOL),
				Ok(FixedI128::saturating_from_rational(3, 100))
			);

			assert_ok!(MarginProtocol::trader_stop_out(Origin::none(), ALICE, MOCK_POOL));

			let event = TestEvent::margin_protocol(RawEvent::TraderStoppedOut(ALICE));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn trader_stop_out_close_bigger_loss_position() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100))
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(70, 60));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(100));
			let loss_position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};

			let bigger_loss_position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(150),
			};

			<Positions<Runtime>>::insert(0, loss_position.clone());
			<Positions<Runtime>>::insert(1, bigger_loss_position.clone());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());

			let snapshot = positions_snapshot(
				2,
				loss_position
					.leveraged_held
					.checked_add(&bigger_loss_position.leveraged_held)
					.unwrap(),
				loss_position
					.leveraged_debits
					.checked_add(&bigger_loss_position.leveraged_debits)
					.unwrap(),
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);

			assert_ok!(MarginProtocol::trader_stop_out(Origin::none(), ALICE, MOCK_POOL));

			// position with bigger loss is closed
			assert!(<PositionsByTrader<Runtime>>::contains_key(ALICE, (MOCK_POOL, 0)));
			assert!(!<PositionsByTrader<Runtime>>::contains_key(ALICE, (MOCK_POOL, 1)));
		});
}

#[test]
fn liquidity_pool_margin_call_and_become_safe_work() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};

			<Positions<Runtime>>::insert(0, position.clone());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			let snapshot = positions_snapshot(
				1,
				position.leveraged_held,
				position.leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);
			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::None),
				Ok((
					FixedI128::saturating_from_integer(1),
					FixedI128::saturating_from_integer(1)
				))
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			assert_noop!(
				MarginProtocol::liquidity_pool_margin_call(Origin::none(), MOCK_POOL),
				Error::<Runtime>::SafePool
			);
			assert_ok!(MarginProtocol::liquidity_pool_become_safe(Origin::none(), MOCK_POOL));

			// ENP 100% == 100%, unsafe
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			assert_ok!(MarginProtocol::liquidity_pool_margin_call(Origin::none(), MOCK_POOL));
			let event = TestEvent::margin_protocol(RawEvent::LiquidityPoolMarginCalled(MOCK_POOL));
			assert!(System::events().iter().any(|record| record.event == event));

			assert_noop!(
				MarginProtocol::liquidity_pool_become_safe(Origin::none(), MOCK_POOL),
				Error::<Runtime>::UnsafePool
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(99, 0));
			assert_ok!(MarginProtocol::liquidity_pool_margin_call(Origin::none(), MOCK_POOL));
			assert_ok!(MarginProtocol::liquidity_pool_become_safe(Origin::none(), MOCK_POOL));
			let event = TestEvent::margin_protocol(RawEvent::LiquidityPoolBecameSafe(MOCK_POOL));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn liquidity_pool_force_close_works() {
	ExtBuilder::default()
		.spread(Price::from_fraction(0.01))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(10_000_00))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(0, 99));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(0, 99));
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_ok!(MarginProtocol::open_position(
				Origin::signed(ALICE),
				MOCK_POOL,
				EUR_USD_PAIR,
				Leverage::LongTwenty,
				balance_saturating_from_integer_currency_cent(10_000_00),
				Price::saturating_from_integer(2)
			));

			assert_eq!(
				MarginProtocol::balances(ALICE, MOCK_POOL),
				fixedi128_saturating_from_integer_currency_cent(10_000_00)
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			assert_noop!(
				MarginProtocol::liquidity_pool_force_close(Origin::none(), MOCK_POOL),
				Error::<Runtime>::NotReachedRiskThreshold
			);

			// Open position spread is 100
			// Current price is 2, close position spread is 200.
			// So liquidity remain 300. Total penalty is 200*2 = 400.
			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(2, 1)));
			// ENP 50% < 99%, unsafe
			assert_ok!(MarginProtocol::liquidity_pool_force_close(Origin::none(), MOCK_POOL));

			let event = TestEvent::margin_protocol(RawEvent::LiquidityPoolForceClosed(MOCK_POOL));
			assert!(System::events().iter().any(|record| record.event == event));

			assert_eq!(
				MarginProtocol::balances(ALICE, MOCK_POOL),
				fixedi128_saturating_from_integer_currency_cent(19_700_00)
			);
			assert_eq!(MockLiquidityPools::liquidity(MOCK_POOL), 0);
			assert_eq!(
				LiquidityCurrency::total_balance(&TREASURY_ACCOUNT),
				300_000000000000000000
			);
		});
}

#[test]
fn open_long_position_works() {
	ExtBuilder::default()
		// USD/JPY = 107
		.price(CurrencyId::FJPY, (1, 107))
		// EUR/JPY = 140.9 => EUR/USD = 140.9/107
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_ok!(MarginProtocol::open_position(
				Origin::signed(ALICE),
				MOCK_POOL,
				EUR_JPY_PAIR,
				Leverage::LongTwenty,
				balance_saturating_from_integer_currency_cent(100_000_00),
				Price::saturating_from_integer(142)
			));

			let position = {
				let mut p = eur_jpy_long();
				// with higher precision level
				p.leveraged_debits = FixedI128::from_inner(-14104090_000000000732500000);
				p.margin_held = FixedI128::from_inner(6590_696261682242990228);
				p
			};
			let id = 0;
			assert_eq!(MarginProtocol::positions(id), Some(position));
			assert_eq!(MarginProtocol::positions_by_trader(ALICE, (MOCK_POOL, id)), Some(()));
			assert_eq!(
				MarginProtocol::positions_by_pool(MOCK_POOL, (EUR_JPY_PAIR, id)),
				Some(())
			);
			assert_eq!(MarginProtocol::next_position_id(), 1);

			let event = TestEvent::margin_protocol(RawEvent::PositionOpened(
				ALICE,
				0,
				MOCK_POOL,
				EUR_JPY_PAIR,
				Leverage::LongTwenty,
				balance_saturating_from_integer_currency_cent(100_000_00),
				// price: 141.0409
				Price::from_inner(141_040900000000007325),
			));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn open_short_position_works() {
	ExtBuilder::default()
		// USD/JPY = 106
		.price(CurrencyId::FJPY, (1, 106))
		// EUR/JPY = 141.9 => EUR/USD = 141.9/106
		.price(CurrencyId::FEUR, (1419, 1060))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_ok!(MarginProtocol::open_position(
				Origin::signed(ALICE),
				MOCK_POOL,
				EUR_JPY_PAIR,
				Leverage::ShortTwenty,
				balance_saturating_from_integer_currency_cent(100_000_00),
				Price::saturating_from_integer(141)
			));

			let position = {
				let mut p = eur_jpy_short();
				// with higher precision level
				p.leveraged_debits = FixedI128::from_inner(14175810_000000000585600000);
				p.margin_held = FixedI128::from_inner(6686_702830188679240668);
				p
			};
			assert_eq!(MarginProtocol::positions(0), Some(position));
		});
}

#[test]
fn open_position_fails_if_trader_margin_called() {
	ExtBuilder::default()
		// USD/JPY = 107
		.price(CurrencyId::FJPY, (1, 107))
		// EUR/JPY = 140.9 => EUR/USD = 140.9/107
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			<MarginCalledTraders<Runtime>>::insert(ALICE, MOCK_POOL, ());
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::MarginCalledTrader
			);
		});
}

#[test]
fn open_position_fails_if_pool_margin_called() {
	ExtBuilder::default()
		// USD/JPY = 107
		.price(CurrencyId::FJPY, (1, 107))
		// EUR/JPY = 140.9 => EUR/USD = 140.9/107
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			MarginCalledPools::insert(MOCK_POOL, ());
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::MarginCalledPool
			);
		});
}

#[test]
fn open_position_fails_if_no_base_price() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::NoPrice
			);
		});
}

#[test]
fn open_position_fails_if_no_quote_price() {
	ExtBuilder::default()
		.price(CurrencyId::FJPY, (1, 107))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::NoPrice
			);
		});
}

#[test]
fn open_long_position_fails_if_market_price_too_high() {
	ExtBuilder::default()
		// USD/JPY = 107
		.price(CurrencyId::FJPY, (1, 107))
		// EUR/JPY = 140.9 => EUR/USD = 140.9/107
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(141)
				),
				Error::<Runtime>::MarketPriceTooHigh
			);
		});
}

#[test]
fn open_short_position_fails_if_market_price_too_low() {
	ExtBuilder::default()
		// USD/JPY = 106
		.price(CurrencyId::FJPY, (1, 106))
		// EUR/JPY = 141.9 => EUR/USD = 141.9/106
		.price(CurrencyId::FEUR, (1419, 1060))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::ShortTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::MarketPriceTooLow
			);
		});
}

#[test]
fn open_position_fails_if_leveraged_debits_out_of_bound() {
	ExtBuilder::default()
		.price(CurrencyId::FJPY, (1, 1))
		.price(CurrencyId::FEUR, (2, 1))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, Balance::max_value())
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, FixedI128::max_value());
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					u128::max_value() / 2 + 1,
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::NumOutOfBound
			);
		});
}

#[test]
fn open_position_fails_if_insufficient_free_margin() {
	ExtBuilder::default()
		.price(CurrencyId::FJPY, (1, 1))
		.price(CurrencyId::FEUR, (2, 1))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, Balance::max_value())
		.build()
		.execute_with(|| {
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwo,
					1,
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::InsufficientFreeMargin
			);
		});
}

#[test]
fn open_position_fails_if_would_reach_enp_threshold() {
	ExtBuilder::default()
		// USD/JPY = 107
		.price(CurrencyId::FJPY, (1, 107))
		// EUR/JPY = 140.9 => EUR/USD = 140.9/107
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(659_00))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_JPY_PAIR, risk_threshold(10, 5));
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::PoolWouldBeUnsafe
			);
		});
}

#[test]
fn open_position_fails_if_would_reach_ell_threshold() {
	ExtBuilder::default()
		// USD/JPY = 107
		.price(CurrencyId::FJPY, (1, 107))
		// EUR/JPY = 140.9 => EUR/USD = 140.9/107
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(659_00))
		.build()
		.execute_with(|| {
			set_ell_risk_threshold(EUR_JPY_PAIR, risk_threshold(10, 5));
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::PoolWouldBeUnsafe
			);
		});
}

#[test]
fn open_position_fails_if_run_out_of_position_id() {
	ExtBuilder::default()
		// USD/JPY = 107
		.price(CurrencyId::FJPY, (1, 107))
		// EUR/JPY = 140.9 => EUR/USD = 140.9/107
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			NextPositionId::put(PositionId::max_value());
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::NoAvailablePositionId
			);
		});
}

#[test]
fn free_margin_cannot_be_used_across_pool() {
	ExtBuilder::default()
		// USD/JPY = 107
		.price(CurrencyId::FJPY, (1, 107))
		// EUR/JPY = 140.9 => EUR/USD = 140.9/107
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(
				ALICE,
				MOCK_POOL_1,
				fixedi128_saturating_from_integer_currency_cent(10_000_00),
			);
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_saturating_from_integer_currency_cent(100_000_00),
					Price::saturating_from_integer(142)
				),
				Error::<Runtime>::InsufficientFreeMargin
			);
		});
}

#[test]
fn close_loss_position_works() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			let position = eur_usd_long_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position.clone());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			let snapshot = positions_snapshot(
				1,
				position.leveraged_held,
				position.leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot.clone());

			assert_ok!(MarginProtocol::close_position(
				Origin::signed(ALICE),
				id,
				Price::saturating_from_rational(11, 10)
			));

			// realized math
			assert_eq!(
				MarginProtocol::balances(ALICE, MOCK_POOL),
				FixedI128::from_inner(9415301035390000000000)
			);
			assert_eq!(MockLiquidityPools::liquidity(MOCK_POOL), 100584698964610000000000);
			assert_eq!(
				LiquidityCurrency::free_balance(&MarginProtocol::account_id()),
				9415301035390000000000
			);

			// position removed
			assert!(MarginProtocol::positions(id).is_none());
			assert_eq!(MarginProtocol::positions_by_trader(ALICE, (MOCK_POOL, id)), None);
			assert_eq!(MarginProtocol::positions_by_pool(MOCK_POOL, (EUR_USD_PAIR, id)), None);

			let event = TestEvent::margin_protocol(RawEvent::PositionClosed(
				ALICE,
				id,
				MOCK_POOL,
				Price::saturating_from_rational(11988, 10000),
			));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn close_loss_position_realizing_part_on_not_enough_equity() {
	ExtBuilder::default()
		.module_balance(fixedi128_saturating_from_integer_currency_cent(1_00))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(1_000_00))
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (10, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(1_00));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTen,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(10_00),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100_00),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(1_00),
			};
			<Positions<Runtime>>::insert(0, position.clone());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			let snapshot = positions_snapshot(
				1,
				position.leveraged_held,
				position.leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot.clone());

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(1, 1)));

			assert_ok!(MarginProtocol::close_position(
				Origin::signed(ALICE),
				0,
				Price::saturating_from_integer(0)
			));
			assert_eq!(
				MockLiquidityPools::liquidity(MOCK_POOL),
				balance_saturating_from_integer_currency_cent(1_001_00)
			);
			assert_eq!(MarginProtocol::balances(&ALICE, MOCK_POOL), FixedI128::zero());
		});
}

#[test]
fn close_loss_position_owning_and_repayment() {
	ExtBuilder::default()
		.module_balance(fixedi128_saturating_from_integer_currency_cent(2_00))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(1_000_00))
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.accumulated_swap_rate(JPY_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (10, 1))
		.price(CurrencyId::FJPY, (10, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(2_00));

			// position with 90 dollars loss
			let loss_position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTen,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(10_00),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100_00),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(1_00),
			};
			// position with 45 dollars profit
			let profit_position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: JPY_USD_PAIR,
				leverage: Leverage::LongFive,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(5_00),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-5_00),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(1_00),
			};
			<Positions<Runtime>>::insert(0, loss_position.clone());
			<Positions<Runtime>>::insert(1, profit_position.clone());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());

			PositionsSnapshots::insert(
				MOCK_POOL,
				EUR_USD_PAIR,
				positions_snapshot(
					1,
					loss_position.leveraged_held,
					loss_position.leveraged_debits,
					FixedI128::saturating_from_integer(0),
					FixedI128::saturating_from_integer(0),
				),
			);
			PositionsSnapshots::insert(
				MOCK_POOL,
				JPY_USD_PAIR,
				positions_snapshot(
					1,
					profit_position.leveraged_held,
					profit_position.leveraged_debits,
					FixedI128::saturating_from_integer(0),
					FixedI128::saturating_from_integer(0),
				),
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(1, 1)));
			assert!(MarginProtocol::equity_of_trader(&ALICE, MOCK_POOL).unwrap() < FixedI128::zero());

			// Balance: FixedI128(2.000000000000000000)
			// Free margin: Ok(FixedI128(-45.000000000000000000))
			// Unrealized PL: Ok(FixedI128(-45.000000000000000000))
			// Equity: Ok(FixedI128(-43.000000000000000000))
			// Margin level: Ok(FixedI128(-0.409523809523809523))

			assert_ok!(MarginProtocol::close_position(
				Origin::signed(ALICE),
				0,
				Price::saturating_from_integer(0)
			));

			// realizable = $47; $2 paid to pool; -$45 owning.
			assert_eq!(
				MockLiquidityPools::liquidity(MOCK_POOL),
				balance_saturating_from_integer_currency_cent(1_002_00)
			);
			assert_eq!(
				MarginProtocol::balances(&ALICE, MOCK_POOL),
				FixedI128::saturating_from_integer(-45),
			);

			// realizable = $45; $45 repayment.
			assert_ok!(MarginProtocol::close_position(
				Origin::signed(ALICE),
				1,
				Price::saturating_from_integer(0)
			));
			assert_eq!(
				MockLiquidityPools::liquidity(MOCK_POOL),
				balance_saturating_from_integer_currency_cent(1_002_00)
			);
			assert_eq!(MarginProtocol::balances(&ALICE, MOCK_POOL), FixedI128::zero(),);
		});
}

#[test]
fn close_profit_position_works() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			let position = eur_usd_long_2();
			let id = 0;
			<Positions<Runtime>>::insert(id, position.clone());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			let snapshot = positions_snapshot(
				1,
				position.leveraged_held,
				position.leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot.clone());
			assert_eq!(
				MarginProtocol::pool_positions_snapshots(MOCK_POOL, EUR_USD_PAIR),
				snapshot
			);

			assert_ok!(MarginProtocol::close_position(
				Origin::signed(ALICE),
				id,
				Price::saturating_from_rational(11, 10)
			));

			let snapshot = positions_snapshot(
				0,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			assert_eq!(
				MarginProtocol::pool_positions_snapshots(MOCK_POOL, EUR_USD_PAIR),
				snapshot
			);
			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::None),
				Ok((FixedI128::max_value(), FixedI128::max_value(),))
			);
			assert_eq!(
				MarginProtocol::balances(ALICE, MOCK_POOL),
				FixedI128::from_inner(10438691023010000000000)
			);
			assert_eq!(MockLiquidityPools::liquidity(MOCK_POOL), 99561308976990000000000);
			assert_eq!(
				LiquidityCurrency::free_balance(&MarginProtocol::account_id()),
				10438691023010000000000
			);
		});
}

#[test]
fn close_position_fails_if_position_not_found() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::saturating_from_rational(11, 10)),
				Error::<Runtime>::PositionNotFound
			);
		});
}

#[test]
fn close_position_fails_if_position_not_opened_by_trader() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			let position = eur_usd_long_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(BOB), 0, Price::saturating_from_rational(11, 10)),
				Error::<Runtime>::PositionNotOpenedByTrader
			);
		});
}

#[test]
fn close_position_fails_if_unrealized_out_of_bound() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		.price(CurrencyId::FEUR, (u128::max_value(), 1))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			let position = eur_usd_long_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::saturating_from_rational(11, 10)),
				Error::<Runtime>::NumOutOfBound
			);
		});
}

#[test]
fn close_position_fails_if_no_base_price() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			let position = eur_jpy_long();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::saturating_from_rational(1410, 1070)),
				Error::<Runtime>::NoPrice
			);
		});
}

#[test]
fn close_position_fails_if_no_quote_price() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		.price(CurrencyId::FJPY, (1, 107))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			let position = eur_jpy_long();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::saturating_from_rational(1390, 1070)),
				Error::<Runtime>::NoPrice
			);
		});
}

#[test]
fn close_long_position_fails_if_market_price_too_low() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			let position = eur_usd_long_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::saturating_from_rational(12, 10)),
				Error::<Runtime>::MarketPriceTooLow
			);
		});
}

#[test]
fn close_short_position_fails_if_market_price_too_high() {
	let alice_initial = fixedi128_saturating_from_integer_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, alice_initial);

			let position = eur_usd_short_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::saturating_from_rational(12, 10)),
				Error::<Runtime>::MarketPriceTooHigh
			);
		});
}

#[test]
fn deposit_works() {
	ExtBuilder::default().alice_balance(1000).build().execute_with(|| {
		assert_eq!(LiquidityCurrency::free_balance(&ALICE), 1000);
		assert_eq!(LiquidityCurrency::free_balance(&MarginProtocol::account_id()), 0);

		assert_ok!(MarginProtocol::deposit(Origin::signed(ALICE), MOCK_POOL, 500));

		assert_eq!(LiquidityCurrency::free_balance(&ALICE), 500);
		assert_eq!(LiquidityCurrency::free_balance(&MarginProtocol::account_id()), 500);
		assert_eq!(MarginProtocol::balances(&ALICE, MOCK_POOL), FixedI128::from_inner(500));

		let event = TestEvent::margin_protocol(RawEvent::Deposited(ALICE, MOCK_POOL, 500));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn deposit_fails_if_transfer_err() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			MarginProtocol::deposit(Origin::signed(ALICE), MOCK_POOL, 500),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
	});
}

#[test]
fn withdraw_works() {
	ExtBuilder::default()
		.module_balance(FixedI128::from_inner(1000))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, FixedI128::from_inner(1000));
			assert_ok!(MarginProtocol::withdraw(Origin::signed(ALICE), MOCK_POOL, 500));

			let event = TestEvent::margin_protocol(RawEvent::Withdrew(ALICE, MOCK_POOL, 500));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn trader_can_withdraw_unrealized_profit() {
	ExtBuilder::default()
		.module_balance(fixedi128_saturating_from_integer_currency_cent(10_00))
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(10_00),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(50),
			};
			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());

			assert_eq!(
				MarginProtocol::free_margin(&ALICE, MOCK_POOL),
				Ok(fixedi128_saturating_from_integer_currency_cent(9_50))
			);
			assert_ok!(MarginProtocol::withdraw(
				Origin::signed(ALICE),
				MOCK_POOL,
				balance_saturating_from_integer_currency_cent(9_50)
			));
		});
}

#[test]
fn withdraw_fails_if_insufficient_free_margin() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			<Balances<Runtime>>::insert(ALICE, MOCK_POOL, fixedi128_saturating_from_integer_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};
			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());

			assert_eq!(
				MarginProtocol::free_margin(&ALICE, MOCK_POOL),
				Ok(fixedi128_saturating_from_integer_currency_cent(0))
			);
			assert_noop!(
				MarginProtocol::withdraw(
					Origin::signed(ALICE),
					MOCK_POOL,
					balance_saturating_from_integer_currency_cent(1)
				),
				Error::<Runtime>::InsufficientFreeMargin
			);
		});
}

#[test]
fn offchain_worker_should_work() {
	let mut ext = ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(200_00))
		.build();

	let (offchain, _state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(3, 1));
		set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(10, 2));
		set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(50, 20));
		<Balances<Runtime>>::insert(
			&ALICE,
			MOCK_POOL,
			fixedi128_saturating_from_integer_currency_cent(10_00),
		);
		assert_eq!(
			MarginProtocol::margin_level(&ALICE, MOCK_POOL).ok().unwrap(),
			FixedI128::max_value()
		);

		assert_ok!(MarginProtocol::open_position(
			Origin::signed(ALICE),
			MOCK_POOL,
			EUR_USD_PAIR,
			Leverage::LongTwenty,
			balance_saturating_from_integer_currency_cent(200_00),
			Price::saturating_from_integer(100)
		));

		assert_eq!(
			MarginProtocol::margin_level(&ALICE, MOCK_POOL).ok().unwrap(),
			FixedI128::saturating_from_rational(5, 100) // 5%
		);

		// price goes down EUR/USD 0.97/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(97, 100)));

		assert_eq!(
			MarginProtocol::margin_level(&ALICE, MOCK_POOL).ok().unwrap(),
			FixedI128::saturating_from_rational(2, 100) // 2%
		);

		assert_ok!(MarginProtocol::offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let trader_margin_call = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*trader_margin_call).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::trader_margin_call(ALICE, MOCK_POOL))
		);

		// price goes down to EUR/USD 0.96/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(96, 100)));

		assert_eq!(
			MarginProtocol::margin_level(&ALICE, MOCK_POOL).ok().unwrap(),
			FixedI128::saturating_from_rational(1, 100) // 1%
		);

		assert_ok!(MarginProtocol::offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let trader_stop_out_call = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*trader_stop_out_call).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::trader_stop_out(ALICE, MOCK_POOL))
		);

		// price goes up to EUR/USD 1.1/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(110, 100)));

		<MarginCalledTraders<Runtime>>::insert(ALICE, MOCK_POOL, ());

		assert_ok!(MarginProtocol::offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let trader_become_safe = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*trader_become_safe).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::trader_become_safe(ALICE, MOCK_POOL))
		);

		<MarginCalledTraders<Runtime>>::remove(ALICE, MOCK_POOL);

		// price goes up to EUR/USD 1.5/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(150, 100)));

		assert_ok!(MarginProtocol::offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let liquidity_pool_margin_call = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*liquidity_pool_margin_call).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::liquidity_pool_margin_call(MOCK_POOL))
		);

		// price goes up to EUR/USD 1.8/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(180, 100)));

		assert_ok!(MarginProtocol::offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let liquidity_pool_force_close = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*liquidity_pool_force_close).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::liquidity_pool_force_close(MOCK_POOL))
		);

		// price goes down to EUR/USD 1.1/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(110, 100)));

		MarginCalledPools::insert(MOCK_POOL, ());

		assert_ok!(MarginProtocol::offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let liquidity_pool_become_safe = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*liquidity_pool_become_safe).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::liquidity_pool_become_safe(MOCK_POOL))
		);

		MarginCalledPools::remove(MOCK_POOL);
	});
}

#[test]
fn liquidity_pool_manager_can_remove_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert!(<MarginProtocol as BaseLiquidityPoolManager<LiquidityPoolId, Balance>>::can_remove(MOCK_POOL));

		<Positions<Runtime>>::insert(0, eur_jpy_long());
		PositionsByPool::insert(MOCK_POOL, (EUR_JPY_PAIR, 0), ());
		let snapshot = positions_snapshot(
			1,
			eur_jpy_long().leveraged_held,
			eur_jpy_long().leveraged_debits,
			FixedI128::saturating_from_integer(0),
			FixedI128::saturating_from_integer(0),
		);
		PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);
		assert!(!<MarginProtocol as BaseLiquidityPoolManager<
			LiquidityPoolId,
			Balance,
		>>::can_remove(MOCK_POOL));
	});
}

#[test]
fn liquidity_pool_manager_get_required_deposit_works() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(0))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(90, 0));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixedi128_saturating_from_integer_currency_cent(100),
				leveraged_debits: fixedi128_saturating_from_integer_currency_cent(-100),
				open_accumulated_swap_rate: FixedI128::saturating_from_integer(1),
				margin_held: fixedi128_saturating_from_integer_currency_cent(100),
			};
			let id = 0;
			<Positions<Runtime>>::insert(id, position.clone());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, id), ());
			let snapshot = positions_snapshot(
				1,
				position.leveraged_held,
				position.leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);

			// need deposit because of ENP
			assert_eq!(
				MarginProtocol::pool_required_deposit(MOCK_POOL),
				Some(fixedi128_saturating_from_integer_currency_cent(100)),
			);

			// need deposit because of ELL
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(80, 0));
			assert_eq!(
				MarginProtocol::pool_required_deposit(MOCK_POOL),
				Some(fixedi128_saturating_from_integer_currency_cent(90)),
			);

			// no need to deposit
			MockLiquidityPools::set_mock_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100));
			assert_eq!(
				MarginProtocol::pool_required_deposit(MOCK_POOL),
				Some(fixedi128_saturating_from_integer_currency_cent(0)),
			);
		});
}

#[test]
fn trader_open_positions_limit() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(1000_00))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(90, 0));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			// give alice $100
			<Balances<Runtime>>::insert(
				&ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(100_00),
			);

			// trader has no open positions
			assert_eq!(
				<PositionsByTrader<Runtime>>::iter_prefix(&ALICE)
					.filter(|((p, _), _)| *p == MOCK_POOL)
					.count(),
				0
			);

			// reach the limit of 200 open positions for a trader
			(0..200u64).for_each(|position_id| {
				<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, position_id), ());
			});

			// trader has 200 open positions
			assert_eq!(
				<PositionsByTrader<Runtime>>::iter_prefix(&ALICE)
					.filter(|((p, _), _)| *p == MOCK_POOL)
					.count(),
				<Runtime as Config>::GetTraderMaxOpenPositions::get()
			);

			// try open another position
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_USD_PAIR,
					Leverage::LongTen,
					balance_saturating_from_integer_currency_cent(10_00),
					Price::saturating_from_integer(100)
				),
				Error::<Runtime>::CannotOpenMorePosition
			);
		});
}

#[test]
fn pool_open_positions_limit() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(1000_00))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(90, 0));
			// give alice $100
			<Balances<Runtime>>::insert(
				&ALICE,
				MOCK_POOL,
				fixedi128_saturating_from_integer_currency_cent(100_00),
			);

			// pool & pair has no open positions
			assert_eq!(
				PositionsByPool::iter_prefix(MOCK_POOL)
					.filter(|((p, _), _)| *p == EUR_USD_PAIR)
					.count(),
				0
			);

			// reach the limit of 300 open positions for a pool & pair
			for _ in 0..300u64 {
				let _ = MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_USD_PAIR,
					Leverage::LongTen,
					balance_saturating_from_integer_currency_cent(1_00),
					Price::saturating_from_integer(100),
				);
			}

			// pool & pair has 1000 open positions
			assert_eq!(
				PositionsByPool::iter_prefix(MOCK_POOL)
					.filter(|((p, _), _)| *p == EUR_USD_PAIR)
					.count(),
				<Runtime as Config>::GetTraderMaxOpenPositions::get()
			);

			assert_eq!(
				PositionsSnapshots::get(MOCK_POOL, EUR_USD_PAIR).positions_count as usize,
				<Runtime as Config>::GetTraderMaxOpenPositions::get()
			);

			// try open another position
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_USD_PAIR,
					Leverage::LongTen,
					balance_saturating_from_integer_currency_cent(10_00),
					Price::saturating_from_integer(100)
				),
				Error::<Runtime>::CannotOpenMorePosition
			);
		});
}

#[test]
fn set_trading_pair_risk_threshold_works() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(1000_00))
		.build()
		.execute_with(|| {
			assert_eq!(
				MarginProtocol::trader_risk_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(0, 0)
			);
			assert_eq!(
				MarginProtocol::liquidity_pool_ell_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(0, 0)
			);
			assert_eq!(
				MarginProtocol::liquidity_pool_enp_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(0, 0)
			);

			assert_ok!(MarginProtocol::set_trading_pair_risk_threshold(
				Origin::signed(UpdateOrigin::get()),
				EUR_USD_PAIR,
				None,
				None,
				None
			));
			let event =
				TestEvent::margin_protocol(RawEvent::TradingPairRiskThresholdSet(EUR_USD_PAIR, None, None, None));
			assert!(System::events().iter().any(|record| record.event == event));

			assert_ok!(MarginProtocol::set_trading_pair_risk_threshold(
				Origin::signed(UpdateOrigin::get()),
				EUR_USD_PAIR,
				Some(risk_threshold(1, 2)),
				None,
				None
			));
			let event = TestEvent::margin_protocol(RawEvent::TradingPairRiskThresholdSet(
				EUR_USD_PAIR,
				Some(risk_threshold(1, 2)),
				None,
				None,
			));
			assert!(System::events().iter().any(|record| record.event == event));
			assert_eq!(
				MarginProtocol::trader_risk_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(1, 2)
			);
			assert_eq!(
				MarginProtocol::liquidity_pool_enp_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(0, 0)
			);
			assert_eq!(
				MarginProtocol::liquidity_pool_ell_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(0, 0)
			);

			assert_ok!(MarginProtocol::set_trading_pair_risk_threshold(
				Origin::signed(UpdateOrigin::get()),
				EUR_USD_PAIR,
				None,
				Some(risk_threshold(3, 4)),
				None
			));
			let event = TestEvent::margin_protocol(RawEvent::TradingPairRiskThresholdSet(
				EUR_USD_PAIR,
				None,
				Some(risk_threshold(3, 4)),
				None,
			));
			assert!(System::events().iter().any(|record| record.event == event));
			assert_eq!(
				MarginProtocol::trader_risk_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(1, 2)
			);
			assert_eq!(
				MarginProtocol::liquidity_pool_enp_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(3, 4)
			);
			assert_eq!(
				MarginProtocol::liquidity_pool_ell_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(0, 0)
			);

			assert_ok!(MarginProtocol::set_trading_pair_risk_threshold(
				Origin::signed(UpdateOrigin::get()),
				EUR_USD_PAIR,
				None,
				None,
				Some(risk_threshold(5, 6))
			));
			let event = TestEvent::margin_protocol(RawEvent::TradingPairRiskThresholdSet(
				EUR_USD_PAIR,
				None,
				None,
				Some(risk_threshold(5, 6)),
			));
			assert!(System::events().iter().any(|record| record.event == event));
			assert_eq!(
				MarginProtocol::trader_risk_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(1, 2)
			);
			assert_eq!(
				MarginProtocol::liquidity_pool_enp_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(3, 4)
			);
			assert_eq!(
				MarginProtocol::liquidity_pool_ell_threshold(EUR_USD_PAIR).unwrap(),
				risk_threshold(5, 6)
			);
		});
}

#[test]
fn ensure_can_enable_trading_pair_works() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(1000_00))
		.build()
		.execute_with(|| {
			assert_noop!(
				MarginProtocol::ensure_can_enable_trading_pair(MOCK_POOL, JPY_USD_PAIR),
				Error::<Runtime>::NoRiskThreshold
			);

			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			let snapshot = positions_snapshot(
				1,
				eur_usd_long_1().leveraged_held,
				eur_usd_long_1().leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);

			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::OpenPosition(eur_usd_long_1())),
				Ok((
					FixedI128::from_inner(0_107101500000000000),
					FixedI128::from_inner(0_107101500000000000)
				))
			);

			assert_ok!(MarginProtocol::ensure_can_enable_trading_pair(MOCK_POOL, EUR_USD_PAIR));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::ensure_can_enable_trading_pair(MOCK_POOL, EUR_USD_PAIR),
				Error::<Runtime>::PoolWouldBeUnsafe
			);
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(5, 5));
			assert_ok!(MarginProtocol::ensure_can_enable_trading_pair(MOCK_POOL, EUR_USD_PAIR));
		});
}

#[test]
fn open_position_check_trader_works() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.price(CurrencyId::FJPY, (1, 105))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(1000_00))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(3, 5));
			set_trader_risk_threshold(EUR_JPY_PAIR, risk_threshold(5, 3));
			set_trader_risk_threshold(JPY_USD_PAIR, risk_threshold(6, 7));

			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_jpy_short());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());

			assert_eq!(
				MarginProtocol::risk_threshold_of_trader(&ALICE, MOCK_POOL),
				risk_threshold(5, 5)
			);

			assert_eq!(
				MarginProtocol::check_trader(&ALICE, MOCK_POOL, Action::OpenPosition(eur_usd_long_1())),
				Ok(Risk::None)
			);

			assert_eq!(
				MarginProtocol::check_trader(&ALICE, MOCK_POOL, Action::OpenPosition(jpy_usd_long_1())),
				Ok(Risk::StopOut)
			);
		});
}

#[test]
fn open_position_check_pool_works() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.price(CurrencyId::FJPY, (70589, 10000))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(100_000_000_00))
		.build()
		.execute_with(|| {
			set_enp_risk_threshold(EUR_USD_PAIR, risk_threshold(30, 50));
			set_enp_risk_threshold(EUR_JPY_PAIR, risk_threshold(50, 30));
			set_enp_risk_threshold(JPY_USD_PAIR, risk_threshold(60, 70));
			set_ell_risk_threshold(JPY_USD_PAIR, risk_threshold(80, 90));
			set_ell_risk_threshold(EUR_USD_PAIR, risk_threshold(10, 20));

			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_jpy_short());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_JPY_PAIR, 1), ());
			let snapshot = positions_snapshot(
				1,
				eur_usd_long_1().leveraged_held,
				eur_usd_long_1().leveraged_debits,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_USD_PAIR, snapshot);
			let snapshot = positions_snapshot(
				1,
				FixedI128::saturating_from_integer(0),
				FixedI128::saturating_from_integer(0),
				eur_jpy_short().leveraged_held,
				eur_jpy_short().leveraged_debits,
			);
			PositionsSnapshots::insert(MOCK_POOL, EUR_JPY_PAIR, snapshot);

			assert_eq!(
				MarginProtocol::enp_and_ell_risk_threshold_of_pool(MOCK_POOL),
				(risk_threshold(50, 50), risk_threshold(10, 20))
			);

			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::OpenPosition(eur_usd_short_1())),
				Ok((
					FixedI128::from_inner(0_182650303333333332),
					FixedI128::from_inner(0_182650303333333332)
				))
			);

			assert_eq!(
				MarginProtocol::check_pool(MOCK_POOL, Action::OpenPosition(eur_usd_short_1())),
				Ok(Risk::StopOut)
			);

			assert_eq!(
				MarginProtocol::enp_and_ell_with_action(MOCK_POOL, Action::OpenPosition(jpy_usd_long_1())),
				Ok((
					FixedI128::from_inner(0_060487576858117431),
					FixedI128::from_inner(0_060487576858117431)
				))
			);

			assert_eq!(
				MarginProtocol::check_pool(MOCK_POOL, Action::OpenPosition(jpy_usd_long_1())),
				Ok(Risk::StopOut)
			);
		});
}

#[test]
fn risk_threshold_of_trader_is_per_pool() {
	ExtBuilder::default()
		.spread(Price::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, FixedI128::saturating_from_integer(1))
		.accumulated_swap_rate(EUR_JPY_PAIR, FixedI128::saturating_from_integer(1))
		.price(CurrencyId::FEUR, (1, 1))
		.price(CurrencyId::FJPY, (1, 105))
		.pool_liquidity(MOCK_POOL, balance_saturating_from_integer_currency_cent(1000_00))
		.build()
		.execute_with(|| {
			set_trader_risk_threshold(EUR_USD_PAIR, risk_threshold(3, 5));
			set_trader_risk_threshold(EUR_JPY_PAIR, risk_threshold(5, 3));
			set_trader_risk_threshold(JPY_USD_PAIR, risk_threshold(6, 7));

			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_jpy_short());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());

			assert_eq!(
				MarginProtocol::risk_threshold_of_trader(&ALICE, MOCK_POOL),
				risk_threshold(5, 5)
			);
			assert_eq!(
				MarginProtocol::risk_threshold_of_trader(&ALICE, MOCK_POOL_1),
				risk_threshold(0, 0)
			);
		});
}
