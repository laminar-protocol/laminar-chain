#![cfg(test)]

use crate::{
	mock::{new_test_ext, ModuleLiquidityPools, Origin},
	LiquidityPool,
};

use sr_primitives::Perbill;
use support::assert_ok;

#[test]
fn should_create_pool() {
	new_test_ext().execute_with(|| {
		let alice: u64 = 1;

		let liquidity_pool = LiquidityPool {
			bid_spread: Perbill::zero(),
			ask_spread: Perbill::zero(),
			additional_collateral_ratio: None,
		};

		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice)));
		assert_eq!(ModuleLiquidityPools::liquidity_pools(1), liquidity_pool);
		assert_eq!(ModuleLiquidityPools::owners(1), alice);
		assert_eq!(ModuleLiquidityPools::next_pool_id(), 2);
	});
}

#[test]
fn is_owner_should_work() {
	new_test_ext().execute_with(|| {
		let alice: u64 = 1;
		assert_ok!(ModuleLiquidityPools::create_pool(Origin::signed(alice)));
		assert_eq!(ModuleLiquidityPools::is_owner(1, alice), true);
		assert_eq!(ModuleLiquidityPools::is_owner(2, alice), false);
	});
}
