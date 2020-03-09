#![cfg(test)]

use crate::{
	mock::{new_test_ext, AccountId, ModuleLiquidityPools, Origin, Runtime, ALICE, BOB},
	Error, Fixed128, LiquidityPoolOption, TradingPair,
};
use sp_std::num::NonZeroI128;

use frame_support::{assert_noop, assert_ok};
use primitives::{CurrencyId, Leverage, Leverages};
use sp_runtime::{PerThing, Permill};
use traits::{LiquidityPools, MarginProtocolLiquidityPools, SyntheticProtocolLiquidityPools};

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
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Leverage::ShortTen | Leverage::LongFive,
		));
		assert_eq!(
			ModuleLiquidityPools::is_enabled(0, CurrencyId::AUSD, Leverage::ShortTen),
			true
		);
		assert_eq!(
			ModuleLiquidityPools::is_enabled(0, CurrencyId::AUSD, Leverage::LongFive),
			true
		);
		assert_eq!(
			ModuleLiquidityPools::is_enabled(0, CurrencyId::AUSD, Leverage::ShortFifty),
			false
		);

		assert_eq!(
			<ModuleLiquidityPools as LiquidityPools<AccountId>>::is_allowed_position(
				0,
				CurrencyId::AUSD,
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
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
		assert_eq!(ModuleLiquidityPools::next_pool_id(), 1);
	});
}

#[test]
fn should_disable_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Leverage::ShortTen | Leverage::LongFive,
		));
		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD),
			Some(LiquidityPoolOption {
				bid_spread: Permill::zero(),
				ask_spread: Permill::zero(),
				additional_collateral_ratio: None,
				enabled_trades: Leverage::ShortTen | Leverage::LongFive,
				synthetic_enabled: false,
			})
		);
		assert_ok!(ModuleLiquidityPools::disable_pool(Origin::signed(ALICE), 0));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
	})
}

#[test]
fn should_remove_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(ModuleLiquidityPools::balances(&0), 1000);
		assert_ok!(ModuleLiquidityPools::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
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
	})
}

#[test]
fn should_withdraw_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
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
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Permill::from_percent(80),
			Permill::from_percent(60)
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Permill::from_percent(80),
			ask_spread: Permill::from_percent(60),
			additional_collateral_ratio: None,
			enabled_trades: Leverages::none(),
			synthetic_enabled: false,
		};

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD),
			Some(pool_option)
		);

		assert_eq!(
			<ModuleLiquidityPools as LiquidityPools<AccountId>>::get_bid_spread(0, CurrencyId::AUSD),
			Some(Permill::from_percent(80))
		);
		assert_eq!(
			<ModuleLiquidityPools as LiquidityPools<AccountId>>::get_ask_spread(0, CurrencyId::AUSD),
			Some(Permill::from_percent(60))
		);
	})
}

#[test]
fn should_set_max_spread() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
		// no max spread
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Permill::one(),
			Permill::one()
		));

		// set max spread to 30%
		assert_ok!(ModuleLiquidityPools::set_max_spread(
			Origin::ROOT,
			CurrencyId::AUSD,
			Permill::from_percent(30)
		));

		assert_noop!(
			ModuleLiquidityPools::set_spread(
				Origin::signed(ALICE),
				0,
				CurrencyId::AUSD,
				Permill::from_percent(31),
				Permill::from_percent(28)
			),
			Error::<Runtime>::SpreadTooHigh
		);

		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Permill::from_percent(28),
			Permill::from_percent(29)
		));

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD),
			Some(LiquidityPoolOption {
				bid_spread: Permill::from_percent(28),
				ask_spread: Permill::from_percent(29),
				additional_collateral_ratio: None,
				enabled_trades: Leverages::none(),
				synthetic_enabled: false,
			})
		);
	});
}

#[test]
fn should_set_additional_collateral_ratio() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
		assert_ok!(ModuleLiquidityPools::set_min_additional_collateral_ratio(
			Origin::ROOT,
			Permill::from_percent(120)
		));
		assert_ok!(ModuleLiquidityPools::set_additional_collateral_ratio(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Some(Permill::from_percent(120)),
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Permill::zero(),
			ask_spread: Permill::zero(),
			additional_collateral_ratio: Some(Permill::from_percent(120)),
			enabled_trades: Leverages::none(),
			synthetic_enabled: false,
		};

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD),
			Some(pool_option)
		);

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::get_additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(120)
		);
		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::get_additional_collateral_ratio(
				0,
				CurrencyId::FJPY
			),
			Permill::from_percent(120)
		);
	})
}

#[test]
fn should_fail_set_additional_collateral_ratio() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::get_additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(0),
		);

		assert_ok!(ModuleLiquidityPools::set_additional_collateral_ratio(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			None,
		));

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::get_additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(0),
		);

		assert_ok!(ModuleLiquidityPools::set_additional_collateral_ratio(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Some(Permill::from_percent(120)),
		));

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::get_additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(120),
		);

		assert_ok!(ModuleLiquidityPools::set_min_additional_collateral_ratio(
			Origin::ROOT,
			Permill::from_percent(150)
		));

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::get_additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(150)
		);

		assert_ok!(ModuleLiquidityPools::set_min_additional_collateral_ratio(
			Origin::ROOT,
			Permill::from_percent(100)
		));

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::get_additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(120)
		);

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::get_additional_collateral_ratio(
				0,
				CurrencyId::FJPY
			),
			Permill::from_percent(100)
		);
	})
}

#[test]
fn should_set_enabled_trades() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Leverage::ShortTen | Leverage::LongFive,
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Permill::zero(),
			ask_spread: Permill::zero(),
			additional_collateral_ratio: None,
			enabled_trades: Leverage::ShortTen | Leverage::LongFive,
			synthetic_enabled: false,
		};

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD),
			Some(pool_option)
		);
	})
}

#[test]
fn should_set_synthetic_enabled() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD), None);
		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::can_mint(0, CurrencyId::AUSD),
			false
		);
		assert_ok!(ModuleLiquidityPools::set_synthetic_enabled(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			true,
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Permill::zero(),
			ask_spread: Permill::zero(),
			additional_collateral_ratio: None,
			enabled_trades: Leverages::none(),
			synthetic_enabled: true,
		};

		assert_eq!(
			ModuleLiquidityPools::liquidity_pool_options(0, CurrencyId::AUSD),
			Some(pool_option)
		);

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::can_mint(0, CurrencyId::AUSD),
			true
		);
	});
}

#[test]
fn should_update_swap() {
	new_test_ext().execute_with(|| {
		let pair = (CurrencyId::LAMI, CurrencyId::AUSD);
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
		let pair = (CurrencyId::LAMI, CurrencyId::AUSD);
		let trading_pair = TradingPair {
			base: pair.0,
			quote: pair.1,
		};
		let rate = Fixed128::from_natural(1);
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::update_swap(Origin::signed(ALICE), 0, pair, rate));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, trading_pair),
			rate
		);
	});
}

#[test]
fn should_get_accumulated_swap() {
	new_test_ext().execute_with(|| {
		let pair = (CurrencyId::AUSD, CurrencyId::FEUR);
		let trading_pair = TradingPair {
			base: pair.0,
			quote: pair.1,
		};
		let rate = Fixed128::from_rational(1, NonZeroI128::new(10).unwrap());
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::update_swap(Origin::signed(ALICE), 0, pair, rate));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_accumulated_swap_rate(
				0,
				trading_pair
			),
			Fixed128::from_natural(0)
		);
		ModuleLiquidityPools::accumulate_swap_rate(0, pair);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_accumulated_swap_rate(
				0,
				trading_pair
			),
			rate
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
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
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
			CurrencyId::AUSD,
			Leverage::ShortFive.into(),
		));
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			CurrencyId::FEUR,
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
		let pair = (CurrencyId::AUSD, CurrencyId::FEUR);
		let trading_pair = TradingPair {
			base: pair.0,
			quote: pair.1,
		};
		let rate = Fixed128::from_rational(1, NonZeroI128::new(10).unwrap());
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::update_swap(Origin::signed(ALICE), 0, pair, rate));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, trading_pair),
			rate
		);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_accumulated_swap_rate(
				0,
				trading_pair
			),
			Fixed128::from_natural(0)
		);
		ModuleLiquidityPools::accumulate_swap_rate(0, pair);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_accumulated_swap_rate(
				0,
				trading_pair
			),
			Fixed128::from_rational(1, NonZeroI128::new(10).unwrap())
		);

		ModuleLiquidityPools::accumulate_swap_rate(0, pair);
		ModuleLiquidityPools::accumulate_swap_rate(0, pair);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_accumulated_swap_rate(
				0,
				trading_pair
			),
			Fixed128::from_rational(3, NonZeroI128::new(10).unwrap())
		);

		let new_rate = Fixed128::from_rational(2, NonZeroI128::new(10).unwrap());
		assert_ok!(ModuleLiquidityPools::update_swap(
			Origin::signed(ALICE),
			0,
			pair,
			new_rate
		));
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_swap_rate(0, trading_pair),
			new_rate
		);

		ModuleLiquidityPools::accumulate_swap_rate(0, pair);
		assert_eq!(
			<ModuleLiquidityPools as MarginProtocolLiquidityPools<AccountId>>::get_accumulated_swap_rate(
				0,
				trading_pair
			),
			Fixed128::from_rational(5, NonZeroI128::new(10).unwrap())
		);
	});
}
