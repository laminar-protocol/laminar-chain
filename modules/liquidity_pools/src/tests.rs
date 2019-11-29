#![cfg(test)]

use crate::{
	mock::{new_test_ext, ModuleLiquidityPools, Origin},
	LiquidityPoolOption,
};

use sr_primitives::Perbill;
use support::assert_ok;

#[test]
fn should_create_pool() {
	new_test_ext().execute_with(|| {
		let alice: u64 = 1;

		let pool_option = LiquidityPoolOption::default();

		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice), 1));
		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(1, 1), pool_option);
		assert_eq!(ModuleLiquidityPools::owners(1), alice);
		assert_eq!(ModuleLiquidityPools::next_pool_id(), 2);
	});
}

#[test]
fn is_owner_should_work() {
	new_test_ext().execute_with(|| {
		let alice: u64 = 1;
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice), 1));
		assert_eq!(ModuleLiquidityPools::is_owner(1, alice), true);
		assert_eq!(ModuleLiquidityPools::is_owner(2, alice), false);
	});
}

#[test]
fn should_disable_pool() {
	new_test_ext().execute_with(|| {
		let alice: u64 = 1;
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice), 1));
		assert_ok!(ModuleLiquidityPools::disable_pool(Origin::signed(alice), 1));
		assert_eq!(
			ModuleLiquidityPools::disable_pool(Origin::signed(alice), 2),
			Err("NoPermission")
		);
	})
}

#[test]
fn should_remove_pool() {
	new_test_ext().execute_with(|| {
		let alice: u64 = 1;
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice), 1));
		assert_ok!(ModuleLiquidityPools::remove_pool(Origin::signed(alice), 1));
		assert_eq!(
			ModuleLiquidityPools::remove_pool(Origin::signed(alice), 2),
			Err("NoPermission")
		);
	})
}

#[test]
fn should_deposit_liquidity() {
	new_test_ext().execute_with(|| {
		let alice: u64 = 1;
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice), 1));
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(Origin::signed(alice), 1, 1000));
	})
}

#[test]
fn should_set_spread() {
	new_test_ext().execute_with(|| {
		let alice: u64 = 1;
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice), 1));
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(alice),
			1,
			1,
			Perbill::one(),
			Perbill::one()
		));

		let pool_option = LiquidityPoolOption {
			bid_spread: Perbill::one(),
			ask_spread: Perbill::one(),
			additional_collateral_ratio: None,
		};

		assert_eq!(ModuleLiquidityPools::liquidity_pool_options(1, 1), pool_option);
	})
}
