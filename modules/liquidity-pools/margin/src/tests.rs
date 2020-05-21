#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok, traits::OnInitialize};
use sp_std::num::NonZeroI128;

use primitives::{CurrencyId, Leverage, Leverages};
use traits::{LiquidityPools, MarginProtocolLiquidityPools};

fn swap_rate(pair: TradingPair, is_long: bool) -> Fixed128 {
	<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair, is_long)
}

fn accumulated_rate(pair: TradingPair, is_long: bool) -> Fixed128 {
	<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_accumulated_swap_rate(0, pair, is_long)
}

#[test]
fn is_enabled_should_work() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
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
fn should_disable_pool() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
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
				bid_spread: 0,
				ask_spread: 0,
				enabled_trades: Leverage::ShortTen | Leverage::LongFive,
			})
		);
		assert_ok!(BaseLiquidityPools::disable_pool(Origin::signed(ALICE), 0));
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
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(&ALICE, 0, 1000));
		assert_eq!(BaseLiquidityPools::balances(&0), 1000);
		assert_ok!(BaseLiquidityPools::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		assert_eq!(BaseLiquidityPools::owners(0), None);
		assert_eq!(BaseLiquidityPools::balances(&0), 0);
		assert_eq!(<ModuleLiquidityPools as LiquidityPools<AccountId>>::liquidity(0), 0);
	})
}

#[test]
fn should_set_spread() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(BaseLiquidityPools::owners(0), Some((ALICE, 0)));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		assert_ok!(ModuleLiquidityPools::set_spread(Origin::signed(ALICE), 0, pair, 80, 60));

		let pool_option = MarginLiquidityPoolOption {
			bid_spread: 80,
			ask_spread: 60,
			enabled_trades: Leverages::none(),
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), Some(pool_option));

		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_bid_spread(0, pair),
			Some(80)
		);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_ask_spread(0, pair),
			Some(60)
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
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(BaseLiquidityPools::owners(0), Some((ALICE, 0)));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		// no max spread
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			pair,
			100,
			100,
		));

		// set max spread to 30%
		assert_ok!(ModuleLiquidityPools::set_max_spread(Origin::ROOT, pair, 30,));

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, pair),
			Some(MarginLiquidityPoolOption {
				bid_spread: 30,
				ask_spread: 30,
				enabled_trades: Leverages::none(),
			})
		);

		assert_ok!(ModuleLiquidityPools::set_spread(Origin::signed(ALICE), 0, pair, 31, 28));

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, pair),
			Some(MarginLiquidityPoolOption {
				bid_spread: 30,
				ask_spread: 28,
				enabled_trades: Leverages::none(),
			})
		);

		assert_ok!(ModuleLiquidityPools::set_spread(Origin::signed(ALICE), 0, pair, 28, 29));

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, pair),
			Some(MarginLiquidityPoolOption {
				bid_spread: 28,
				ask_spread: 29,
				enabled_trades: Leverages::none(),
			})
		);

		assert_ok!(ModuleLiquidityPools::set_max_spread(Origin::ROOT, pair, 20));

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, pair),
			Some(MarginLiquidityPoolOption {
				bid_spread: 20,
				ask_spread: 20,
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
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(BaseLiquidityPools::owners(0), Some((ALICE, 0)));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), None);
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			pair,
			Leverage::ShortTen | Leverage::LongFive,
		));

		let pool_option = MarginLiquidityPoolOption {
			bid_spread: 0,
			ask_spread: 0,
			enabled_trades: Leverage::ShortTen | Leverage::LongFive,
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, pair), Some(pool_option));
	})
}

#[test]
fn should_set_swap_rate() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::LAMI,
			quote: CurrencyId::AUSD,
		};
		let rate = SwapRate {
			long: Fixed128::from_natural(-1),
			short: Fixed128::from_natural(1),
		};
		let bad_rate = SwapRate {
			long: Fixed128::from_natural(-3),
			short: Fixed128::from_natural(3),
		};
		let bad_long_rate = SwapRate {
			long: Fixed128::from_natural(-3),
			short: Fixed128::from_natural(1),
		};
		let bad_short_rate = SwapRate {
			long: Fixed128::from_natural(-1),
			short: Fixed128::from_natural(3),
		};
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::set_swap_rate(Origin::ROOT, pair, rate));
		assert_noop!(
			ModuleLiquidityPools::set_swap_rate(Origin::ROOT, pair, bad_rate),
			Error::<Runtime>::SwapRateTooHigh
		);
		assert_noop!(
			ModuleLiquidityPools::set_swap_rate(Origin::ROOT, pair, bad_long_rate),
			Error::<Runtime>::SwapRateTooHigh
		);
		assert_noop!(
			ModuleLiquidityPools::set_swap_rate(Origin::ROOT, pair, bad_short_rate),
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
		let rate = SwapRate {
			long: Fixed128::from_natural(-1),
			short: Fixed128::from_natural(1),
		};
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::set_swap_rate(Origin::ROOT, pair, rate.clone()));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair, true),
			rate.long
		);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair, false),
			rate.short
		);

		// add additional swap rate
		let rate = Fixed128::from_natural(1);
		assert_ok!(ModuleLiquidityPools::set_additional_swap(
			Origin::signed(ALICE),
			0,
			rate
		));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair, true),
			Fixed128::from_natural(-2)
		);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair, false),
			Fixed128::from_natural(0)
		);

		let rate = Fixed128::from_natural(2);
		assert_ok!(ModuleLiquidityPools::enable_trading_pair(Origin::ROOT, pair));
		assert_ok!(ModuleLiquidityPools::liquidity_pool_enable_trading_pair(
			Origin::signed(ALICE),
			0,
			pair
		));
		assert_ok!(ModuleLiquidityPools::set_additional_swap(
			Origin::signed(ALICE),
			0,
			rate
		));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair, true),
			MaxSwap::get()
		);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, pair, false),
			Fixed128::from_natural(-1)
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
		let rate = SwapRate {
			long: Fixed128::from_rational(-1, NonZeroI128::new(10).unwrap()), // -10%
			short: Fixed128::from_rational(1, NonZeroI128::new(10).unwrap()), // 10%
		};

		assert_ok!(ModuleLiquidityPools::set_accumulate(Origin::ROOT, pair, 1, 0));
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::set_swap_rate(Origin::ROOT, pair, rate.clone()));
		assert_eq!(
			accumulated_rate(pair, true),
			Fixed128::from_natural(0) // 0%
		);
		assert_eq!(
			accumulated_rate(pair, false),
			Fixed128::from_natural(0) // 0%
		);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(1);
		assert_eq!(accumulated_rate(pair, true), rate.long);
		assert_eq!(accumulated_rate(pair, false), rate.short);

		// add additional swap rate
		let rate = Fixed128::from_rational(1, NonZeroI128::new(10).unwrap()); // 10%
		assert_ok!(ModuleLiquidityPools::set_additional_swap(
			Origin::signed(ALICE),
			0,
			rate
		));
		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(1);
		assert_eq!(
			accumulated_rate(pair, true),
			Fixed128::from_rational(-30, NonZeroI128::new(100).unwrap())
		);
		assert_eq!(
			accumulated_rate(pair, false),
			Fixed128::from_rational(10, NonZeroI128::new(100).unwrap())
		);
	});
}

#[test]
fn can_open_position() {
	new_test_ext().execute_with(|| {
		let pair = TradingPair {
			base: CurrencyId::AUSD,
			quote: CurrencyId::FEUR,
		};
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
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
		let rate = SwapRate {
			long: Fixed128::from_rational(-23, NonZeroI128::new(1000).unwrap()), // -2.3%
			short: Fixed128::from_rational(23, NonZeroI128::new(1000).unwrap()), // 2.3%
		};

		assert_ok!(ModuleLiquidityPools::set_accumulate(Origin::ROOT, pair, 1, 0));
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::set_swap_rate(Origin::ROOT, pair, rate.clone()));
		assert_eq!(swap_rate(pair, true), rate.long);
		assert_eq!(swap_rate(pair, false), rate.short);

		let acc = Fixed128::from_natural(0); // 0%
		assert_eq!(accumulated_rate(pair, true), acc);
		assert_eq!(accumulated_rate(pair, false), acc);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(1);
		let long_acc = Fixed128::from_rational(-23, NonZeroI128::new(1000).unwrap()); // -2.3%
		let short_acc = Fixed128::from_rational(23, NonZeroI128::new(1000).unwrap()); // 2.3%
		assert_eq!(accumulated_rate(pair, true), long_acc);
		assert_eq!(accumulated_rate(pair, false), short_acc);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(2);
		let long_acc = Fixed128::from_rational(-46, NonZeroI128::new(1000).unwrap()); // -4.6%
		let short_acc = Fixed128::from_rational(46, NonZeroI128::new(1000).unwrap()); // 4.6%
		assert_eq!(accumulated_rate(pair, true), long_acc);
		assert_eq!(accumulated_rate(pair, false), short_acc);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(3);
		let long_acc = Fixed128::from_rational(-69, NonZeroI128::new(1000).unwrap()); // -6.9%
		let short_acc = Fixed128::from_rational(69, NonZeroI128::new(1000).unwrap()); // 6.9%
		assert_eq!(accumulated_rate(pair, true), long_acc);
		assert_eq!(accumulated_rate(pair, false), short_acc);

		<ModuleLiquidityPools as OnInitialize<u64>>::on_initialize(4);
		let long_acc = Fixed128::from_rational(-92, NonZeroI128::new(1000).unwrap()); // 9.2%
		let short_acc = Fixed128::from_rational(92, NonZeroI128::new(1000).unwrap()); // 9.2%
		assert_eq!(accumulated_rate(pair, true), long_acc);
		assert_eq!(accumulated_rate(pair, false), short_acc);
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
		assert_eq!(ModuleLiquidityPools::enabled_trading_pair(pair), Some(true));
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
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::enable_trading_pair(Origin::ROOT, pair));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_enabled_trading_pair(0, pair), None);
		assert_ok!(ModuleLiquidityPools::liquidity_pool_enable_trading_pair(
			Origin::signed(ALICE),
			0,
			pair
		));
		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_enabled_trading_pair(0, pair),
			Some(true)
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

		assert_eq!(ModuleLiquidityPools::min_leveraged_amount(pool_id), None);
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
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::set_min_leveraged_amount(
			Origin::signed(ALICE),
			pool_id,
			2
		));
		assert_eq!(ModuleLiquidityPools::min_leveraged_amount(pool_id), Some(2));
		assert_eq!(ModuleLiquidityPools::get_min_leveraged_amount(pool_id), 10);

		// non pool owners cannot set min leveraged amount
		assert_noop!(
			ModuleLiquidityPools::set_min_leveraged_amount(Origin::signed(BOB), pool_id, 20),
			Error::<Runtime>::NoPermission
		);
	})
}
