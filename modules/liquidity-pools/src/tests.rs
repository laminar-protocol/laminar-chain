#![cfg(test)]

use crate::{
	mock::{new_test_ext, ModuleLiquidityPools, Origin, ALICE},
	LiquidityPoolOption,
};

use frame_support::assert_ok;
use sp_runtime::Perbill;

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
			Perbill::one(),
			Perbill::one()
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Perbill::one(),
			ask_spread: Perbill::one(),
			additional_collateral_ratio: None,
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(0, 1), Some(pool_option));
	})
}
