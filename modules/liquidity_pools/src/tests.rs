#![cfg(test)]

use crate::{
	mock::{new_test_ext, ModuleLiquidityPools, Origin},
	LiquidityPool,
};

use support::assert_ok;

#[test]
fn should_create_pool() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		let bob = 2;

		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice)));
		assert_eq!(ModuleLiquidityPools::liquidity_pools(1), LiquidityPool::default());
		// pool_id 1 is owned by Alice
		assert_eq!(ModuleLiquidityPools::owners(1), alice);

		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(bob)));
		// pool_id 2 is owned by Bob
		assert_eq!(ModuleLiquidityPools::owners(2), bob);

		assert_eq!(ModuleLiquidityPools::next_pool_id(), 3);
	});
}

#[test]
fn is_owner_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice)));
		assert_eq!(ModuleLiquidityPools::is_owner(1, alice), true);
		assert_eq!(ModuleLiquidityPools::is_owner(2, alice), false);
	});
}
