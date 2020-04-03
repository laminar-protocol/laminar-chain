//! Unit tests for the margin protocol module.

#![cfg(test)]

use super::*;
use mock::*;

use core::num::NonZeroI128;
use frame_support::{assert_noop, assert_ok};
use primitives::Leverage;
use sp_core::offchain::{
	testing::{TestOffchainExt, TestTransactionPoolExt},
	OffchainExt, TransactionPoolExt,
};
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

fn risk_threshold(margin_call_percent: u32, stop_out_percent: u32) -> RiskThreshold {
	RiskThreshold {
		margin_call: Permill::from_percent(margin_call_percent),
		stop_out: Permill::from_percent(stop_out_percent),
	}
}

fn eur_jpy_long() -> Position<Runtime> {
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_JPY_PAIR,
		leverage: Leverage::LongTwenty,
		leveraged_held: Fixed128::from_natural(100_000),
		leveraged_debits: Fixed128::from_natural(-14_104_090),
		leveraged_debits_in_usd: fixed128_from_natural_currency_cent(-131_813_93),
		open_accumulated_swap_rate: Fixed128::from_natural(1),
		margin_held: fixed128_from_natural_currency_cent(6_591_00),
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
		leveraged_debits_in_usd: fixed128_from_natural_currency_cent(133_734_06),
		open_accumulated_swap_rate: Fixed128::from_natural(1),
		margin_held: fixed128_from_natural_currency_cent(6_687_00),
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
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
			assert_eq!(
				MarginProtocol::unrealized_pl_of_trader(&ALICE),
				Ok(Fixed128::from_parts(397454545454545390254))
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
			MarginProtocol::margin_held(&ALICE),
			fixed128_from_natural_currency_cent(13_278_00)
		);
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
		leveraged_debits_in_usd: fixed128_from_natural_currency_cent(-120_420_30),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixed128_from_natural_currency_cent(24_084_00),
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
		leveraged_debits_in_usd: fixed128_from_natural_currency_cent(-119_419_30),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixed128_from_natural_currency_cent(5_971_00),
	}
}

fn eur_usd_short_1() -> Position<Runtime> {
	let open_rate =
		Fixed128::from_natural(1).saturating_add(Fixed128::from_rational(10_96, NonZeroI128::new(100_000_00).unwrap()));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::ShortTen,
		leveraged_held: Fixed128::from_natural(-100_000),
		leveraged_debits: fixed128_from_natural_currency_cent(119_780_10),
		leveraged_debits_in_usd: fixed128_from_natural_currency_cent(119_780_10),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixed128_from_natural_currency_cent(11_978_00),
	}
}

fn eur_usd_short_2() -> Position<Runtime> {
	let open_rate =
		Fixed128::from_natural(1).saturating_add(Fixed128::from_rational(3_65, NonZeroI128::new(100_000_00).unwrap()));
	Position {
		owner: ALICE,
		pool: MOCK_POOL,
		pair: EUR_USD_PAIR,
		leverage: Leverage::ShortFifty,
		leveraged_held: Fixed128::from_natural(-200_000),
		leveraged_debits: fixed128_from_natural_currency_cent(237_362_40),
		leveraged_debits_in_usd: fixed128_from_natural_currency_cent(237_362_40),
		open_accumulated_swap_rate: open_rate,
		margin_held: fixed128_from_natural_currency_cent(4_747_00),
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
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
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
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(120_000_00));
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 2), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 3), ());
			assert_eq!(
				MarginProtocol::equity_of_trader(&ALICE),
				Ok(fixed128_from_natural_currency_cent(116_665_86))
			);
		});
}

#[test]
fn margin_level_works() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(120_000_00));
			<Positions<Runtime>>::insert(0, eur_usd_long_1());
			<Positions<Runtime>>::insert(1, eur_usd_long_2());
			<Positions<Runtime>>::insert(2, eur_usd_short_1());
			<Positions<Runtime>>::insert(3, eur_usd_short_2());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 1), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 2), ());
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 3), ());
			assert_eq!(
				MarginProtocol::margin_level(&ALICE),
				// 19.44%
				Ok(Fixed128::from_parts(195426060513372176))
			);
		});
}

#[test]
fn margin_level_without_any_opened_position_is_max() {
	ExtBuilder::default().build().execute_with(|| {
		<Balances<Runtime>>::insert(ALICE, Fixed128::from_parts(1));
		assert_eq!(MarginProtocol::margin_level(&ALICE), Ok(Fixed128::max_value()));
	});
}

#[test]
fn ensure_trader_safe_works() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold(100, 0))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};
			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			assert_eq!(MarginProtocol::margin_level(&ALICE), Ok(Fixed128::from_natural(1)));

			// 100% == 100%, unsafe
			assert_noop!(
				MarginProtocol::_ensure_trader_safe(&ALICE),
				Error::<Runtime>::UnsafeTrader
			);

			// 100% > 99%, safe
			TraderRiskThreshold::put(risk_threshold(99, 0));
			assert_ok!(MarginProtocol::_ensure_trader_safe(&ALICE));
		});
}

#[test]
fn equity_of_pool_works() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
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
			assert_eq!(
				MarginProtocol::_equity_of_pool(MOCK_POOL),
				Ok(fixed128_from_natural_currency_cent(103_334_14))
			);
		});
}

#[test]
fn enp_and_ell_without_new_position_works() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
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

			assert_eq!(
				MarginProtocol::_enp_and_ell(MOCK_POOL, Action::None),
				Ok((
					Fixed128::from_parts(880917181075659681),
					Fixed128::from_parts(289335881335881335)
				))
			);
		});
}

#[test]
fn enp_and_ell_with_new_position_works() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			// enp = ell = 100_000_00 / 120_420_30
			assert_eq!(
				MarginProtocol::_enp_and_ell(MOCK_POOL, Action::OpenPosition(eur_usd_long_1())),
				Ok((
					Fixed128::from_parts(830424770574396509),
					Fixed128::from_parts(830424770574396509)
				))
			);
		});
}

#[test]
fn enp_and_ell_without_position_with_liquidity_works() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
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

			assert_eq!(
				MarginProtocol::_enp_and_ell(MOCK_POOL, Action::Withdraw(balance_from_natural_currency_cent(10))),
				Ok((
					Fixed128::from_parts(880916328581816817),
					Fixed128::from_parts(289335601335601335)
				))
			);
		});
}

#[test]
fn ensure_liquidity_works() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100))
		.liquidity_pool_ell_threshold(risk_threshold(99, 0))
		.liquidity_pool_enp_threshold(risk_threshold(99, 0))
		.build()
		.execute_with(|| {
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};

			<Positions<Runtime>>::insert(0, position);
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_ok!(MarginProtocol::ensure_can_withdraw(MOCK_POOL, 10));

			LiquidityPoolELLThreshold::put(risk_threshold(100, 0));

			assert_noop!(
				MarginProtocol::ensure_can_withdraw(MOCK_POOL, 1),
				Error::<Runtime>::PoolWouldBeUnsafe
			);
		});
}

#[test]
fn ensure_pool_safe_works() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100))
		.liquidity_pool_ell_threshold(risk_threshold(99, 0))
		.liquidity_pool_enp_threshold(risk_threshold(99, 0))
		.build()
		.execute_with(|| {
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};

			// with new position
			assert_eq!(
				MarginProtocol::_enp_and_ell(MOCK_POOL, Action::OpenPosition(position.clone())),
				Ok((Fixed128::from_natural(1), Fixed128::from_natural(1)))
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			assert_ok!(MarginProtocol::_ensure_pool_safe(
				MOCK_POOL,
				Action::OpenPosition(position.clone()),
			));

			// ENP 100% == 100%, unsafe
			LiquidityPoolENPThreshold::put(risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::_ensure_pool_safe(MOCK_POOL, Action::OpenPosition(position.clone())),
				Error::<Runtime>::PoolWouldBeUnsafe
			);

			// ELL 100% == 100%, unsafe
			LiquidityPoolENPThreshold::put(risk_threshold(99, 0));
			LiquidityPoolELLThreshold::put(risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::_ensure_pool_safe(MOCK_POOL, Action::OpenPosition(position.clone())),
				Error::<Runtime>::PoolWouldBeUnsafe
			);

			// without new position
			<Positions<Runtime>>::insert(0, position);
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			LiquidityPoolELLThreshold::put(risk_threshold(99, 0));
			assert_eq!(
				MarginProtocol::_enp_and_ell(MOCK_POOL, Action::None),
				Ok((Fixed128::from_natural(1), Fixed128::from_natural(1)))
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			assert_ok!(MarginProtocol::_ensure_pool_safe(MOCK_POOL, Action::None));

			// ENP 100% == 100%, unsafe
			LiquidityPoolENPThreshold::put(risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::_ensure_pool_safe(MOCK_POOL, Action::None),
				Error::<Runtime>::UnsafePool
			);

			// ELL 100% == 100%, unsafe
			LiquidityPoolENPThreshold::put(risk_threshold(99, 0));
			LiquidityPoolELLThreshold::put(risk_threshold(100, 0));
			assert_noop!(
				MarginProtocol::_ensure_pool_safe(MOCK_POOL, Action::None),
				Error::<Runtime>::UnsafePool
			);
		});
}

#[test]
fn trader_margin_call_should_work() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold(5, 3))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};

			assert_eq!(MarginProtocol::margin_level(&ALICE), Ok(Fixed128::max_value()));

			// without position
			assert_noop!(
				MarginProtocol::trader_margin_call(Origin::NONE, ALICE),
				Error::<Runtime>::SafeTrader
			);

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			assert_eq!(MarginProtocol::margin_level(&ALICE), Ok(Fixed128::from_natural(1)));

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(1, 20)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE),
				Ok(Fixed128::from_rational(5, NonZeroI128::new(100).unwrap()))
			);

			assert_ok!(MarginProtocol::trader_margin_call(Origin::NONE, ALICE));
		});
}

#[test]
fn trader_become_safe_should_work() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold(5, 3))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};

			// without position
			assert_ok!(MarginProtocol::trader_become_safe(Origin::NONE, ALICE));

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			assert_eq!(MarginProtocol::margin_level(&ALICE), Ok(Fixed128::from_natural(1)));

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(4, 100)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE),
				Ok(Fixed128::from_rational(4, NonZeroI128::new(100).unwrap()))
			);
			assert_ok!(MarginProtocol::trader_margin_call(Origin::NONE, ALICE));
			assert_noop!(
				MarginProtocol::trader_become_safe(Origin::NONE, ALICE),
				Error::<Runtime>::UnsafeTrader
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(5, 100)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE),
				Ok(Fixed128::from_rational(5, NonZeroI128::new(100).unwrap()))
			);
			assert_noop!(
				MarginProtocol::trader_become_safe(Origin::NONE, ALICE),
				Error::<Runtime>::UnsafeTrader
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(6, 100)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE),
				Ok(Fixed128::from_rational(6, NonZeroI128::new(100).unwrap()))
			);
			assert_ok!(MarginProtocol::trader_become_safe(Origin::NONE, ALICE));
		});
}
#[test]
fn trader_liquidate_should_work() {
	ExtBuilder::default()
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100))
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold(5, 3))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};

			// without position
			assert_noop!(
				MarginProtocol::trader_liquidate(Origin::NONE, ALICE),
				Error::<Runtime>::NotReachedRiskThreshold
			);

			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			assert_eq!(MarginProtocol::margin_level(&ALICE), Ok(Fixed128::from_natural(1)));

			// trader_liquidate without trader_margin_call
			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(3, 100)));
			assert_eq!(
				MarginProtocol::margin_level(&ALICE),
				Ok(Fixed128::from_rational(3, NonZeroI128::new(100).unwrap()))
			);

			assert_ok!(MarginProtocol::trader_liquidate(Origin::NONE, ALICE));
		});
}

#[test]
fn liquidity_pool_margin_call_and_become_safe_work() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100))
		.liquidity_pool_ell_threshold(risk_threshold(99, 0))
		.liquidity_pool_enp_threshold(risk_threshold(99, 0))
		.build()
		.execute_with(|| {
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};

			<Positions<Runtime>>::insert(0, position);
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());
			assert_eq!(
				MarginProtocol::_enp_and_ell(MOCK_POOL, Action::None),
				Ok((Fixed128::from_natural(1), Fixed128::from_natural(1)))
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			assert_noop!(
				MarginProtocol::liquidity_pool_margin_call(Origin::NONE, MOCK_POOL),
				Error::<Runtime>::SafePool
			);
			assert_ok!(MarginProtocol::liquidity_pool_become_safe(Origin::NONE, MOCK_POOL));

			// ENP 100% == 100%, unsafe
			LiquidityPoolENPThreshold::put(risk_threshold(100, 0));
			assert_ok!(MarginProtocol::liquidity_pool_margin_call(Origin::NONE, MOCK_POOL));
			let event = TestEvent::margin_protocol(RawEvent::LiquidityPoolMarginCalled(MOCK_POOL));
			assert!(System::events().iter().any(|record| record.event == event));

			assert_noop!(
				MarginProtocol::liquidity_pool_become_safe(Origin::NONE, MOCK_POOL),
				Error::<Runtime>::UnsafePool
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			LiquidityPoolENPThreshold::put(risk_threshold(99, 0));
			assert_ok!(MarginProtocol::liquidity_pool_margin_call(Origin::NONE, MOCK_POOL));
			assert_ok!(MarginProtocol::liquidity_pool_become_safe(Origin::NONE, MOCK_POOL));
			let event = TestEvent::margin_protocol(RawEvent::LiquidityPoolBecameSafe(MOCK_POOL));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn liquidity_pool_liquidate_works() {
	ExtBuilder::default()
		.spread(Permill::from_rational_approximation(1, 100u32))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(10_000_00))
		.liquidity_pool_ell_threshold(risk_threshold(0, 99))
		.liquidity_pool_enp_threshold(risk_threshold(0, 99))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_ok!(MarginProtocol::open_position(
				Origin::signed(ALICE),
				MOCK_POOL,
				EUR_USD_PAIR,
				Leverage::LongTwenty,
				balance_from_natural_currency_cent(10_000_00),
				Price::from_natural(2)
			));

			assert_eq!(
				MarginProtocol::balances(ALICE),
				fixed128_from_natural_currency_cent(10_000_00)
			);

			// ENP 100% > 99%, ELL 100% > 99%, safe
			assert_noop!(
				MarginProtocol::liquidity_pool_liquidate(Origin::NONE, MOCK_POOL),
				Error::<Runtime>::NotReachedRiskThreshold
			);

			// Open position spread is 100
			// Current price is 20, close position spread is 200.
			// So liquidity remain 300. Total penalty is 200*2 = 400.
			MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(2, 1)));
			// ENP 50% < 99%, unsafe
			assert_ok!(MarginProtocol::liquidity_pool_liquidate(Origin::NONE, MOCK_POOL));

			let event = TestEvent::margin_protocol(RawEvent::LiquidityPoolLiquidated(MOCK_POOL));
			assert!(System::events().iter().any(|record| record.event == event));

			assert_eq!(
				MarginProtocol::balances(ALICE),
				fixed128_from_natural_currency_cent(19_700_00)
			);
			assert_eq!(
				MockLiquidityPools::liquidity(MOCK_POOL),
				balance_from_natural_currency_cent(0)
			);
			assert_eq!(
				OrmlTokens::total_balance(CurrencyId::AUSD, &TREASURY_ACCOUNT),
				balance_from_natural_currency_cent(300_00)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_ok!(MarginProtocol::open_position(
				Origin::signed(ALICE),
				MOCK_POOL,
				EUR_JPY_PAIR,
				Leverage::LongTwenty,
				balance_from_natural_currency_cent(100_000_00),
				Price::from_natural(142)
			));

			let position = {
				let mut p = eur_jpy_long();
				// with higher precision level
				p.leveraged_debits = Fixed128::from_parts(-14104090000000000732500000);
				p.leveraged_debits_in_usd = Fixed128::from_parts(-131813925233644859804554);
				p.margin_held = Fixed128::from_parts(6590696261682242990227);
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
				MOCK_POOL,
				EUR_JPY_PAIR,
				Leverage::LongTwenty,
				balance_from_natural_currency_cent(100_000_00),
				// price: 141.0409
				Price::from_parts(141040900000000007325),
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_ok!(MarginProtocol::open_position(
				Origin::signed(ALICE),
				MOCK_POOL,
				EUR_JPY_PAIR,
				Leverage::ShortTwenty,
				balance_from_natural_currency_cent(100_000_00),
				Price::from_natural(141)
			));

			let position = {
				let mut p = eur_jpy_short();
				// with higher precision level
				p.leveraged_debits = Fixed128::from_parts(14175810000000000585500000);
				p.leveraged_debits_in_usd = Fixed128::from_parts(133734056603773584812414);
				p.margin_held = Fixed128::from_parts(6686702830188679240620);
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			<MarginCalledTraders<Runtime>>::insert(ALICE, ());
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(142)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			MarginCalledPools::insert(MOCK_POOL, ());
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(142)
				),
				Error::<Runtime>::MarginCalledPool
			);
		});
}

#[test]
fn open_position_fails_if_no_base_price() {
	ExtBuilder::default()
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(142)
				),
				Error::<Runtime>::NoPrice
			);
		});
}

#[test]
fn open_position_fails_if_no_quote_price() {
	ExtBuilder::default()
		.price(CurrencyId::FJPY, (1, 107))
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(142)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(141)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::ShortTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(142)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, Balance::max_value())
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, Fixed128::max_value());
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					u128::max_value() / 2 + 1,
					Price::from_natural(142)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
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
					Price::from_natural(142)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(659_00))
		.liquidity_pool_enp_threshold(risk_threshold(10, 5))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(142)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(659_00))
		.liquidity_pool_ell_threshold(risk_threshold(10, 5))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(142)
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
		.accumulated_swap_rate(EUR_JPY_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(10_000_00));
			NextPositionId::put(PositionId::max_value());
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_JPY_PAIR,
					Leverage::LongTwenty,
					balance_from_natural_currency_cent(100_000_00),
					Price::from_natural(142)
				),
				Error::<Runtime>::NoAvailablePositionId
			);
		});
}

#[test]
fn close_loss_position_works() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			let position = eur_usd_long_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_ok!(MarginProtocol::close_position(
				Origin::signed(ALICE),
				id,
				Price::from_rational(11, 10)
			));

			// realized math
			assert_eq!(
				MarginProtocol::balances(ALICE),
				fixed128_from_natural_currency_cent(9422_83)
			);
			assert_eq!(
				MockLiquidityPools::liquidity(MOCK_POOL),
				balance_from_natural_currency_cent(100577_17)
			);
			assert_eq!(
				OrmlTokens::free_balance(CurrencyId::AUSD, &MarginProtocol::account_id()),
				balance_from_natural_currency_cent(9422_83)
			);

			// position removed
			assert!(MarginProtocol::positions(id).is_none());
			assert_eq!(MarginProtocol::positions_by_trader(ALICE, (MOCK_POOL, id)), None);
			assert_eq!(MarginProtocol::positions_by_pool(MOCK_POOL, (EUR_USD_PAIR, id)), None);

			let event =
				TestEvent::margin_protocol(RawEvent::PositionClosed(ALICE, id, Price::from_rational(11988, 10000)));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn close_profit_position_works() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			let position = eur_usd_long_2();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_ok!(MarginProtocol::close_position(
				Origin::signed(ALICE),
				id,
				Price::from_rational(11, 10)
			));

			assert_eq!(
				MarginProtocol::balances(ALICE),
				fixed128_from_natural_currency_cent(10442_27)
			);
			assert_eq!(
				MockLiquidityPools::liquidity(MOCK_POOL),
				balance_from_natural_currency_cent(99557_73)
			);
			assert_eq!(
				OrmlTokens::free_balance(CurrencyId::AUSD, &MarginProtocol::account_id()),
				balance_from_natural_currency_cent(10442_27)
			);
		});
}

#[test]
fn close_position_fails_if_position_not_found() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::from_rational(11, 10)),
				Error::<Runtime>::PositionNotFound
			);
		});
}

#[test]
fn close_position_fails_if_position_not_opened_by_trader() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			let position = eur_usd_long_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(BOB), 0, Price::from_rational(11, 10)),
				Error::<Runtime>::PositionNotOpenedByTrader
			);
		});
}

#[test]
fn close_position_fails_if_unrealized_out_of_bound() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		.price(CurrencyId::FEUR, (u128::max_value(), 1))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			let position = eur_usd_long_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::from_rational(11, 10)),
				Error::<Runtime>::NumOutOfBound
			);
		});
}

#[test]
fn close_position_fails_if_no_base_price() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		.price(CurrencyId::FEUR, (1409, 1070))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			let position = eur_jpy_long();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::from_rational(1410, 1070)),
				Error::<Runtime>::NoPrice
			);
		});
}

#[test]
fn close_position_fails_if_no_quote_price() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		.price(CurrencyId::FJPY, (1, 107))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			let position = eur_jpy_long();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::from_rational(1390, 1070)),
				Error::<Runtime>::NoPrice
			);
		});
}

#[test]
fn close_long_position_fails_if_market_price_too_low() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			let position = eur_usd_long_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::from_rational(12, 10)),
				Error::<Runtime>::MarketPriceTooLow
			);
		});
}

#[test]
fn close_short_position_fails_if_market_price_too_high() {
	let alice_initial = fixed128_from_natural_currency_cent(10_000_00);
	ExtBuilder::default()
		.module_balance(alice_initial)
		// EUR/USD = 1.2
		.price(CurrencyId::FEUR, (12, 10))
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100_000_00))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, alice_initial);

			let position = eur_usd_short_1();
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, 0), ());

			assert_noop!(
				MarginProtocol::close_position(Origin::signed(ALICE), 0, Price::from_rational(12, 10)),
				Error::<Runtime>::MarketPriceTooHigh
			);
		});
}

#[test]
fn deposit_works() {
	ExtBuilder::default().alice_balance(1000).build().execute_with(|| {
		assert_eq!(OrmlTokens::free_balance(CurrencyId::AUSD, &ALICE), 1000);
		assert_eq!(
			OrmlTokens::free_balance(CurrencyId::AUSD, &MarginProtocol::account_id()),
			0
		);

		assert_ok!(MarginProtocol::deposit(Origin::signed(ALICE), 500));

		assert_eq!(OrmlTokens::free_balance(CurrencyId::AUSD, &ALICE), 500);
		assert_eq!(
			OrmlTokens::free_balance(CurrencyId::AUSD, &MarginProtocol::account_id()),
			500
		);
		assert_eq!(MarginProtocol::balances(&ALICE), Fixed128::from_parts(500));

		let event = TestEvent::margin_protocol(RawEvent::Deposited(ALICE, 500));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn deposit_fails_if_transfer_err() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			MarginProtocol::deposit(Origin::signed(ALICE), 500),
			orml_tokens::Error::<Runtime>::BalanceTooLow
		);
	});
}

#[test]
fn withdraw_works() {
	ExtBuilder::default()
		.module_balance(Fixed128::from_parts(1000))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, Fixed128::from_parts(1000));
			assert_ok!(MarginProtocol::withdraw(Origin::signed(ALICE), 500));

			let event = TestEvent::margin_protocol(RawEvent::Withdrew(ALICE, 500));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn trader_can_withdraw_unrealized_profit() {
	ExtBuilder::default()
		.module_balance(fixed128_from_natural_currency_cent(10_00))
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold(100, 0))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(10_00),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(50),
			};
			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());

			assert_eq!(
				MarginProtocol::free_margin(&ALICE),
				Ok(fixed128_from_natural_currency_cent(9_50))
			);
			assert_ok!(MarginProtocol::withdraw(
				Origin::signed(ALICE),
				balance_from_natural_currency_cent(9_50)
			));
		});
}

#[test]
fn withdraw_fails_if_insufficient_free_margin() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.trader_risk_threshold(risk_threshold(100, 0))
		.build()
		.execute_with(|| {
			<Balances<Runtime>>::insert(ALICE, fixed128_from_natural_currency_cent(100));
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};
			<Positions<Runtime>>::insert(0, position);
			<PositionsByTrader<Runtime>>::insert(ALICE, (MOCK_POOL, 0), ());

			assert_eq!(
				MarginProtocol::free_margin(&ALICE),
				Ok(fixed128_from_natural_currency_cent(0))
			);
			assert_noop!(
				MarginProtocol::withdraw(Origin::signed(ALICE), balance_from_natural_currency_cent(1)),
				Error::<Runtime>::InsufficientFreeMargin
			);
		});
}

#[test]
fn offchain_worker_should_work() {
	let mut ext = ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(200_00))
		.trader_risk_threshold(risk_threshold(3, 1))
		.liquidity_pool_ell_threshold(risk_threshold(50, 20))
		.liquidity_pool_enp_threshold(risk_threshold(10, 2))
		.build();

	let (offchain, _state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	ext.register_extension(OffchainExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));

	ext.execute_with(|| {
		<Balances<Runtime>>::insert(&ALICE, fixed128_from_natural_currency_cent(10_00));
		assert_eq!(
			MarginProtocol::margin_level(&ALICE).ok().unwrap(),
			Fixed128::max_value()
		);

		assert_ok!(MarginProtocol::open_position(
			Origin::signed(ALICE),
			MOCK_POOL,
			EUR_USD_PAIR,
			Leverage::LongTwenty,
			balance_from_natural_currency_cent(200_00),
			Price::from_natural(100)
		));

		assert_eq!(
			MarginProtocol::margin_level(&ALICE).ok().unwrap(),
			Fixed128::from_rational(5, NonZeroI128::new(100).unwrap()) // 5%
		);

		// price goes down EUR/USD 0.97/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(97, 100)));

		assert_eq!(
			MarginProtocol::margin_level(&ALICE).ok().unwrap(),
			Fixed128::from_rational(2, NonZeroI128::new(100).unwrap()) // 2%
		);

		assert_ok!(MarginProtocol::_offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let trader_margin_call = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*trader_margin_call).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::trader_margin_call(ALICE))
		);

		// price goes down to EUR/USD 0.96/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(96, 100)));

		assert_eq!(
			MarginProtocol::margin_level(&ALICE).ok().unwrap(),
			Fixed128::from_rational(1, NonZeroI128::new(100).unwrap()) // 1%
		);

		assert_ok!(MarginProtocol::_offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let trader_liquidate_call = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*trader_liquidate_call).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::trader_liquidate(ALICE))
		);

		// price goes up to EUR/USD 1.1/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(110, 100)));

		<MarginCalledTraders<Runtime>>::insert(ALICE, ());

		assert_ok!(MarginProtocol::_offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let trader_become_safe = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*trader_become_safe).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::trader_become_safe(ALICE))
		);

		<MarginCalledTraders<Runtime>>::remove(ALICE);

		// price goes up to EUR/USD 1.5/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(150, 100)));

		assert_ok!(MarginProtocol::_offchain_worker(1));

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
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(180, 100)));

		assert_ok!(MarginProtocol::_offchain_worker(1));

		assert_eq!(pool_state.read().transactions.len(), 1);
		let liquidity_pool_liquidate = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());

		let tx = Extrinsic::decode(&mut &*liquidity_pool_liquidate).unwrap();

		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			mock::Call::MarginProtocol(super::Call::liquidity_pool_liquidate(MOCK_POOL))
		);

		// price goes down to EUR/USD 1.1/1
		MockPrices::set_mock_price(CurrencyId::FEUR, Some(FixedU128::from_rational(110, 100)));

		MarginCalledPools::insert(MOCK_POOL, ());

		assert_ok!(MarginProtocol::_offchain_worker(1));

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
		assert!(<MarginProtocol as LiquidityPoolManager<LiquidityPoolId, Balance>>::can_remove(MOCK_POOL));

		<Positions<Runtime>>::insert(0, eur_jpy_long());
		PositionsByPool::insert(MOCK_POOL, (EUR_JPY_PAIR, 0), ());
		assert!(!<MarginProtocol as LiquidityPoolManager<LiquidityPoolId, Balance>>::can_remove(MOCK_POOL));
	});
}

#[test]
fn liquidity_pool_manager_get_required_deposit_works() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(0))
		.liquidity_pool_ell_threshold(risk_threshold(90, 0))
		.liquidity_pool_enp_threshold(risk_threshold(100, 0))
		.build()
		.execute_with(|| {
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: EUR_USD_PAIR,
				leverage: Leverage::LongTwo,
				leveraged_held: fixed128_from_natural_currency_cent(100),
				leveraged_debits: fixed128_from_natural_currency_cent(100),
				leveraged_debits_in_usd: fixed128_from_natural_currency_cent(100),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				margin_held: fixed128_from_natural_currency_cent(100),
			};
			let id = 0;
			<Positions<Runtime>>::insert(id, position);
			PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, id), ());

			// need deposit because of ENP
			assert_eq!(
				MarginProtocol::get_required_deposit(MOCK_POOL),
				Ok(balance_from_natural_currency_cent(100)),
			);

			// need deposit because of ELL
			LiquidityPoolENPThreshold::put(risk_threshold(80, 0));
			assert_eq!(
				MarginProtocol::get_required_deposit(MOCK_POOL),
				Ok(balance_from_natural_currency_cent(90)),
			);

			// no need to deposit
			MockLiquidityPools::set_mock_liquidity(MOCK_POOL, balance_from_natural_currency_cent(100));
			assert_eq!(
				MarginProtocol::get_required_deposit(MOCK_POOL),
				Ok(balance_from_natural_currency_cent(0)),
			);
		});
}

#[test]
fn trader_open_positions_limit() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(1000_00))
		.liquidity_pool_ell_threshold(risk_threshold(90, 0))
		.liquidity_pool_enp_threshold(risk_threshold(100, 0))
		.build()
		.execute_with(|| {
			// give alice $100
			<Balances<Runtime>>::insert(&ALICE, fixed128_from_natural_currency_cent(100_00));

			// trader has no open positions
			assert_eq!(
				<PositionsByTrader<Runtime>>::iter(&ALICE)
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
				<PositionsByTrader<Runtime>>::iter(&ALICE)
					.filter(|((p, _), _)| *p == MOCK_POOL)
					.count(),
				200
			);

			// try open another position
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_USD_PAIR,
					Leverage::LongTen,
					balance_from_natural_currency_cent(10_00),
					Price::from_natural(100)
				),
				Error::<Runtime>::CannotOpenMorePosition
			);
		});
}

#[test]
fn pool_open_positions_limit() {
	ExtBuilder::default()
		.spread(Permill::zero())
		.accumulated_swap_rate(EUR_USD_PAIR, Fixed128::from_natural(1))
		.price(CurrencyId::FEUR, (1, 1))
		.pool_liquidity(MOCK_POOL, balance_from_natural_currency_cent(1000_00))
		.liquidity_pool_ell_threshold(risk_threshold(90, 0))
		.liquidity_pool_enp_threshold(risk_threshold(100, 0))
		.build()
		.execute_with(|| {
			// give alice $100
			<Balances<Runtime>>::insert(&ALICE, fixed128_from_natural_currency_cent(100_00));

			// pool & pair has no open positions
			assert_eq!(
				PositionsByPool::iter(MOCK_POOL)
					.filter(|((p, _), _)| *p == EUR_USD_PAIR)
					.count(),
				0
			);

			// reach the limit of 1000 open positions for a pool & pair
			(0..1000u64).for_each(|position_id| {
				PositionsByPool::insert(MOCK_POOL, (EUR_USD_PAIR, position_id), ());
			});

			// pool & pair has 1000 open positions
			assert_eq!(
				PositionsByPool::iter(MOCK_POOL)
					.filter(|((p, _), _)| *p == EUR_USD_PAIR)
					.count(),
				1000
			);

			// try open another position
			assert_noop!(
				MarginProtocol::open_position(
					Origin::signed(ALICE),
					MOCK_POOL,
					EUR_USD_PAIR,
					Leverage::LongTen,
					balance_from_natural_currency_cent(10_00),
					Price::from_natural(100)
				),
				Error::<Runtime>::CannotOpenMorePosition
			);
		});
}
