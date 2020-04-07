#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use traits::LiquidityPools;

#[test]
fn is_owner_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_eq!(Instance1Module::is_owner(0, &ALICE), true);
		assert_eq!(Instance1Module::is_owner(1, &ALICE), false);
		assert_eq!(
			<Instance1Module as LiquidityPools<AccountId>>::is_owner(1, &ALICE),
			false
		);
	});
}

#[test]
fn should_create_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_eq!(Instance1Module::owners(0), Some((ALICE, 0)));
		assert_eq!(Instance1Module::next_pool_id(), 1);
	});
}

#[test]
fn should_disable_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_ok!(Instance1Module::disable_pool(Origin::signed(ALICE), 0));
	})
}

#[test]
fn should_remove_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_ok!(Instance1Module::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(Instance1Module::owners(0), None);
		assert_eq!(Instance1Module::balances(&0), 0);
		assert_eq!(<Instance1Module as LiquidityPools<AccountId>>::liquidity(0), 0);
	})
}

#[test]
fn should_deposit_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_eq!(Instance1Module::balances(&0), 0);
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_eq!(<Instance1Module as LiquidityPools<AccountId>>::liquidity(0), 1000);
		assert_noop!(
			Instance1Module::deposit_liquidity(Origin::signed(ALICE), 1, 1000),
			Error::<Runtime, Instance1>::PoolNotFound
		);
	})
}

#[test]
fn should_withdraw_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_eq!(Instance1Module::owners(0), Some((ALICE, 0)));
		assert_eq!(Instance1Module::balances(&0), 0);
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_ok!(Instance1Module::withdraw_liquidity(Origin::signed(ALICE), 0, 500));
		assert_eq!(Instance1Module::balances(&0), 500);
		assert_ok!(<Instance1Module as LiquidityPools<AccountId>>::withdraw_liquidity(
			&BOB, 0, 100
		));
		assert_eq!(Instance1Module::balances(&0), 400);
	})
}

#[test]
fn should_fail_withdraw_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_eq!(
			Instance1Module::withdraw_liquidity(Origin::signed(ALICE), 0, 5000),
			Err(Error::<Runtime, Instance1>::CannotWithdrawAmount.into()),
		);

		assert_eq!(
			Instance1Module::withdraw_liquidity(Origin::signed(ALICE), 0, 1000),
			Err(Error::<Runtime, Instance1>::CannotWithdrawExistentialDeposit.into()),
		);

		assert_eq!(Instance1Module::balances(&0), 1000);
	})
}
