#![cfg(test)]

use crate::{
	mock::{new_test_ext, ModuleLiquidityPools, Origin, ALICE},
	LiquidityPoolOption,
};

use frame_support::assert_ok;
use primitives::{Leverage, Leverages};
use sp_runtime::Permill;

#[test]
fn should_create_pool() {
	new_test_ext().execute_with(|| {
		let pool_option = LiquidityPoolOption::default();

		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE), 1));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, 1), Some(pool_option));
		assert_eq!(ModuleLiquidityPools::owners(0), Some(ALICE));
		assert_eq!(ModuleLiquidityPools::next_pool_id(), 1);
	});
}

#[test]
fn is_owner_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE), 1));
		assert_eq!(ModuleLiquidityPools::is_owner(0, ALICE), true);
		assert_eq!(ModuleLiquidityPools::is_owner(1, ALICE), false);
	});
}

#[test]
fn should_disable_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE), 1));
		assert_ok!(ModuleLiquidityPools::disable_pool(Origin::signed(ALICE), 0));
		assert_eq!(
			ModuleLiquidityPools::disable_pool(Origin::signed(ALICE), 2),
			Err("NoPermission")
		);
	})
}

#[test]
fn should_remove_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE), 1));
		assert_ok!(ModuleLiquidityPools::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(
			ModuleLiquidityPools::remove_pool(Origin::signed(ALICE), 2),
			Err("NoPermission")
		);
	})
}

#[test]
fn should_deposit_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE), 1));
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(Origin::signed(ALICE), 1, 1000));
	})
}

#[test]
fn should_set_spread() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE), 1));
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			1,
			Permill::one(),
			Permill::one()
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Permill::one(),
			ask_spread: Permill::one(),
			additional_collateral_ratio: None,
			enabled_longs: Leverages::none(),
			enabled_shorts: Leverages::none(),
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, 1), Some(pool_option));
	})
}

#[test]
fn should_set_additional_collateral_ratio() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE), 1));
		assert_ok!(ModuleLiquidityPools::set_additional_collateral_ratio(
			Origin::signed(ALICE),
			0,
			1,
			Some(Permill::from_percent(120)),
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Permill::zero(),
			ask_spread: Permill::zero(),
			additional_collateral_ratio: Some(Permill::from_percent(120)),
			enabled_longs: Leverages::none(),
			enabled_shorts: Leverages::none(),
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, 1), Some(pool_option));
	})
}

#[test]
fn should_set_enabled_trades() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(ALICE), 1));
		assert_ok!(ModuleLiquidityPools::set_enabled_trades(
			Origin::signed(ALICE),
			0,
			1,
			Leverage::Ten.into(),
			Leverage::Five.into()
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Permill::zero(),
			ask_spread: Permill::zero(),
			additional_collateral_ratio: None,
			enabled_longs: Leverage::Ten.into(),
			enabled_shorts: Leverage::Five.into(),
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, 1), Some(pool_option));
	})
}
