#![cfg(test)]

use crate::{
	mock::{new_test_ext, AccountId, ModuleLiquidityPools, Origin, Runtime, ALICE, BOB},
	Error, Fixed128, MarginLiquidityPoolOption, TradingPair,
};
use sp_std::num::NonZeroI128;

use frame_support::{assert_noop, assert_ok};
use primitives::{CurrencyId, Leverage, Leverages};
use sp_runtime::traits::OnInitialize;
use sp_runtime::{PerThing, Permill};
use traits::{LiquidityPools, MarginProtocolLiquidityPools};

fn swap_rate(pair: TradingPair) -> Fixed128 {
	<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair)
}

fn accumulated_rate(pair: TradingPair) -> Fixed128 {
	<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_accumulated_swap_rate(0, pair)
}

#[test]
fn is_owner_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::is_owner(0, &ALICE), true);
		assert_eq!(ModuleLiquidityPools::is_owner(1, &ALICE), false);
		assert_eq!(
			<ModuleLiquidityPools as LiquidityPools<AccountId>>::is_owner(1, &ALICE),
			false
		);
	});
}

#[test]
fn is_enabled_should_work() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			pair,
			Leverage::ShortTen | Leverage::LongFive,
		));
		assert_eq!(ModuleLiquidityPools::is_enabled(0, pair, Leverage::ShortTen), true);
		assert_eq!(ModuleLiquidityPools::is_enabled(0, pair, Leverage::LongFive), true);
		assert_eq!(ModuleLiquidityPools::is_enabled(0, pair, Leverage::ShortFifty), false);

		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::is_allowed_position(
				0,
				pair,
				Leverage::ShortTen
			),
			true
		);
	});
}

#[test]
fn should_create_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some((ALICE, 0)));
		assert_eq!(ModuleLiquidityPools::next_pool_id(), 1);
	});
}

#[test]
fn should_disable_pool() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			pair,
			Leverage::ShortTen | Leverage::LongFive,
		));
		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, pair),
			Some(MarginLiquidityPoolOption {
				bid_spread: Permill::zero(),
				ask_spread: Permill::zero(),
				enabled_trades: Leverage::ShortTen | Leverage::LongFive,
			})
		);
		assert_ok!(ModuleLiquidityPools::disable_pool(Origin::signed(ALICE), 0));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
	})
}

#[test]
fn should_remove_pool() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(ModuleLiquidityPools::balances(&0), 1000);
		assert_ok!(ModuleLiquidityPools::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		assert_eq!(ModuleLiquidityPools::owners(0), None);
		assert_eq!(ModuleLiquidityPools::balances(&0), 0);
		assert_eq!(<ModuleLiquidityPools as LiquidityPools<AccountId>>::liquidity(0), 0);
	})
}

#[test]
fn should_deposit_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::balances(&0), 0);
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(ModuleLiquidityPools::balances(&0), 1000);
		assert_eq!(<ModuleLiquidityPools as LiquidityPools<AccountId>>::liquidity(0), 1000);
		assert_noop!(
			ModuleLiquidityPools::deposit_liquidity(Origin::signed(ALICE), 1, 1000),
			Error::<Runtime>::PoolNotFound
		);
	})
}

#[test]
fn should_withdraw_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some((ALICE, 0)));
		assert_eq!(ModuleLiquidityPools::balances(&0), 0);
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(ModuleLiquidityPools::balances(&0), 1000);
		assert_ok!(ModuleLiquidityPools::withdraw_liquidity(Origin::signed(ALICE), 0, 500));
		assert_eq!(ModuleLiquidityPools::balances(&0), 500);
		assert_ok!(<ModuleLiquidityPools as LiquidityPools<AccountId>>::withdraw_liquidity(
			&BOB, 0, 100
		));
		assert_eq!(ModuleLiquidityPools::balances(&0), 400);
	})
}

#[test]
fn should_fail_withdraw_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(ModuleLiquidityPools::balances(&0), 1000);
		assert_eq!(
			ModuleLiquidityPools::withdraw_liquidity(Origin::signed(ALICE), 0, 5000),
			Err(Error::<Runtime>::CannotWithdrawAmount.into()),
		);

		assert_eq!(
			ModuleLiquidityPools::withdraw_liquidity(Origin::signed(ALICE), 0, 1000),
			Err(Error::<Runtime>::CannotWithdrawExistentialDeposit.into()),
		);

		assert_eq!(ModuleLiquidityPools::balances(&0), 1000);
	})
}

#[test]
fn should_set_spread() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some((ALICE, 0)));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			pair,
			Permill::from_percent(80),
			Permill::from_percent(60)
		));

		let pool_option = MarginLiquidityPoolOption {
			bid_spread: Permill::from_percent(80),
			ask_spread: Permill::from_percent(60),
			enabled_trades: Leverages::none(),
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), Some(pool_option));

		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_bid_spread(0, pair),
			Some(Permill::from_percent(80))
		);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_ask_spread(0, pair),
			Some(Permill::from_percent(60))
		);
	})
}

#[test]
fn should_set_max_spread() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some((ALICE, 0)));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		// no max spread
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			pair,
			Permill::one(),
			Permill::one()
		));

		// set max spread to 30%
		assert_ok!(ModuleLiquidityPools::set_max_spread(
			Origin::ROOT,
			pair,
			Permill::from_percent(30)
		));

		assert_noop!(
			ModuleLiquidityPools::set_spread(
				Origin::signed(ALICE),
				0,
				pair,
				Permill::from_percent(31),
				Permill::from_percent(28)
			),
			Error::<Runtime>::SpreadTooHigh
		);

		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			pair,
			Permill::from_percent(28),
			Permill::from_percent(29)
		));

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, pair),
			Some(MarginLiquidityPoolOption {
				bid_spread: Permill::from_percent(28),
				ask_spread: Permill::from_percent(29),
				enabled_trades: Leverages::none(),
			})
		);
	});
}

#[test]
fn should_set_enabled_trades() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some((ALICE, 0)));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			pair,
			Leverage::ShortTen | Leverage::LongFive,
		));

		let pool_option = MarginLiquidityPoolOption {
			bid_spread: Permill::zero(),
			ask_spread: Permill::zero(),
			enabled_trades: Leverage::ShortTen | Leverage::LongFive,
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), Some(pool_option));
	})
}

#[test]
fn should_update_swap() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::LAMI,
			quote: CurrencyId::AUSD,
		};
		let rate = Fixed128::from_natural(1);
		let bad_rate = Fixed128::from_natural(3);
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::update_swap(Origin::signed(ALICE), 0, pair, rate));
		assert_noop!(
			ModuleLiquidityPools::update_swap(Origin::signed(ALICE), 0, pair, bad_rate),
			Error::<Runtime>::SwapRateTooHigh
		);
	});
}

#[test]
fn should_get_swap() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::LAMI,
			quote: CurrencyId::AUSD,
		};
		let rate = Fixed128::from_natural(1);
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::update_swap(Origin::signed(ALICE), 0, pair, rate));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair),
			rate
		);
	});
}

#[test]
fn should_get_accumulated_swap() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		let rate = Fixed128::from_rational(1, NonZeroI128::new(10).unwrap()); // 10%

		assert_ok!(ModuleLiquidityPools::set_accumulate(Origin::ROOT, pair, 1, 0));
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::update_swap(Origin::signed(ALICE), 0, pair, rate));
		assert_eq!(
			accumulated_rate(pair),
			Fixed128::from_natural(0) // 0%
		);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(1);
		assert_eq!(accumulated_rate(pair), rate);
	});
}

#[test]
fn can_open_position() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::enable_trading_pair(Origin::ROOT, pair));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_enabled_trading_pair(0, pair), None);
		assert_ok!(ModuleLiquidityPools::liquidity_pool_enable_trading_pair(
			Origin::signed(ALICE),
			0,
			pair
		));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::can_open_position(
				0,
				pair,
				Leverage::ShortFive,
				0
			),
			false
		);

		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			pair,
			Leverage::ShortFive.into(),
		));
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			pair,
			Leverage::ShortFive.into(),
		));

		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::can_open_position(
				0,
				pair,
				Leverage::ShortFive,
				0
			),
			true
		);
	});
}

#[test]
fn should_update_accumulated_rate() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		let rate = Fixed128::from_rational(23, NonZeroI128::new(1000).unwrap()); // 2.3%

		assert_ok!(ModuleLiquidityPools::set_accumulate(Origin::ROOT, pair, 1, 0));
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::update_swap(Origin::signed(ALICE), 0, pair, rate));
		assert_eq!(swap_rate(pair), rate);

		let acc = Fixed128::from_natural(0); // 0%
		assert_eq!(accumulated_rate(pair), acc);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(1);
		let acc = Fixed128::from_rational(23, NonZeroI128::new(1000).unwrap()); // 2.3%
		assert_eq!(accumulated_rate(pair), acc);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(2);
		let acc = Fixed128::from_rational(46529, NonZeroI128::new(1000000).unwrap()); // 4.6529%
		assert_eq!(accumulated_rate(pair), acc);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(3);
		let acc = Fixed128::from_rational(70599167i128, NonZeroI128::new(1000000000).unwrap()); // 7.0599%
		assert_eq!(accumulated_rate(pair), acc);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(4);
		let acc = Fixed128::from_rational(95222947841i128, NonZeroI128::new(1000000000000).unwrap()); // 9.5223%
		assert_eq!(accumulated_rate(pair), acc);
	});
}

#[test]
fn should_enable_disable_trading_pairs() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_eq!(ModuleLiquidityPools::enabled_trading_pair(pair), None);
		assert_ok!(ModuleLiquidityPools::enable_trading_pair(Origin::ROOT, pair));
		assert_eq!(ModuleLiquidityPools::enabled_trading_pair(pair), Some(pair));
		assert_ok!(ModuleLiquidityPools::disable_trading_pair(Origin::ROOT, pair));
		assert_eq!(ModuleLiquidityPools::enabled_trading_pair(pair), None);
	})
}

#[test]
fn liquidity_provider_should_enable_disable_trading_pairs() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::enable_trading_pair(Origin::ROOT, pair));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_enabled_trading_pair(0, pair), None);
		assert_ok!(ModuleLiquidityPools::liquidity_pool_enable_trading_pair(
			Origin::signed(ALICE),
			0,
			pair
		));
		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_enabled_trading_pair(0, pair),
			Some(pair)
		);
		assert_ok!(ModuleLiquidityPools::liquidity_pool_disable_trading_pair(
			Origin::signed(ALICE),
			0,
			pair
		));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_enabled_trading_pair(0, pair), None);
	})
}

#[test]
fn should_set_default_min_leveraged_amount() {
	new_test_ext().execute_with(|| {
		let pool_id = 0;

		assert_eq!(ModuleLiquidityPools::default_min_leveraged_amount(), 0);
		assert_eq!(ModuleLiquidityPools::get_min_leveraged_amount(pool_id), 0);

		// set default min leveraged amount
		assert_ok!(ModuleLiquidityPools::set_default_min_leveraged_amount(Origin::ROOT, 10));
		assert_eq!(ModuleLiquidityPools::default_min_leveraged_amount(), 10);
		assert_eq!(ModuleLiquidityPools::get_min_leveraged_amount(pool_id), 10);
	})
}

#[test]
fn should_set_min_leveraged_amount() {
	new_test_ext().execute_with(|| {
		let pool_id = 0;

		assert_eq!(ModuleLiquidityPools::min_leveraged_amount(pool_id), 0);
		assert_eq!(ModuleLiquidityPools::get_min_leveraged_amount(pool_id), 0);

		// set default min leveraged amount
		assert_ok!(ModuleLiquidityPools::set_default_min_leveraged_amount(Origin::ROOT, 10));
		assert_eq!(ModuleLiquidityPools::get_min_leveraged_amount(pool_id), 10);

		// pool not created yet
		assert_noop!(
			ModuleLiquidityPools::set_min_leveraged_amount(Origin::signed(ALICE), pool_id, 10),
			Error::<Runtime>::NoPermission
		);

		// create pool and set min leveraged amount
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::set_min_leveraged_amount(
			Origin::signed(ALICE),
			pool_id,
			2
		));
		assert_eq!(ModuleLiquidityPools::min_leveraged_amount(pool_id), 2);

		// non pool owners cannot set min leveraged amount
		assert_noop!(
			ModuleLiquidityPools::set_min_leveraged_amount(Origin::signed(BOB), pool_id, 20),
			Error::<Runtime>::NoPermission
		);
	})
}
