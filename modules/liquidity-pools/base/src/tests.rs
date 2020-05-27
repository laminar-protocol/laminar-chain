#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use traits::LiquidityPools;

#[test]
fn is_owner_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert!(Instance1Module::is_owner(0, &ALICE));
		assert!(!Instance1Module::is_owner(1, &ALICE));
		assert!(!<Instance1Module as LiquidityPools<AccountId>>::is_owner(1, &ALICE));
	});
}

#[test]
fn pool_exits_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Instance1Module::pool_exists(0), false);
		assert_eq!(Instance1Module::pool_exists(1), false);
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_eq!(Instance1Module::pool_exists(0), true);
		assert_eq!(Instance1Module::pool_exists(1), false);
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

#[test]
fn should_set_identity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));

		let mut identity = IdentityRequest {
			legal: "laminar".as_bytes().to_vec(),
			display: vec![0; 201],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};
		assert_noop!(
			Instance1Module::set_identity(Origin::signed(ALICE), 0, identity.clone()),
			Error::<Runtime, Instance1>::IdentityInfoTooLong
		);

		identity.display = vec![];

		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));

		identity.display = "Open finance platform".as_bytes().to_vec();
		assert_ok!(Instance1Module::set_identity(Origin::signed(ALICE), 0, identity));
	})
}

#[test]
fn should_verify_identity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));

		let identity = IdentityRequest {
			legal: "laminar".as_bytes().to_vec(),
			display: vec![],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));

		// verify
		assert_eq!(Instance1Module::identity_infos(0).unwrap().verify_status, false);
		assert_eq!(Instance1Module::balances(&0), 0);
		assert_ok!(Instance1Module::verify_identity(Origin::ROOT, 0));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_eq!(Instance1Module::identity_infos(0).unwrap().deposit_status, true);
		assert_eq!(Instance1Module::identity_infos(0).unwrap().verify_status, true);
		// verify then modify
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(Instance1Module::identity_infos(0).unwrap().verify_status, false);
		assert_eq!(Instance1Module::identity_infos(0).unwrap().deposit_status, true);
		assert_ok!(Instance1Module::verify_identity(Origin::ROOT, 0));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_eq!(Instance1Module::identity_infos(0).unwrap().verify_status, true);
	})
}

#[test]
fn should_clear_identity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));

		let identity = IdentityRequest {
			legal: "laminar".as_bytes().to_vec(),
			display: vec![],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};

		// clear without verify
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(Instance1Module::identity_infos(0).unwrap().verify_status, false);
		assert_ok!(Instance1Module::clear_identity(Origin::ROOT, 0));
		assert_noop!(
			Instance1Module::clear_identity(Origin::ROOT, 0),
			Error::<Runtime, Instance1>::IdentityNotFound
		);
		assert_eq!(Instance1Module::identity_infos(0), None);

		// verify then clear
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(Instance1Module::balances(&0), 0);
		assert_ok!(Instance1Module::verify_identity(Origin::ROOT, 0));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_eq!(Instance1Module::identity_infos(0).unwrap().verify_status, true);
		assert_ok!(Instance1Module::clear_identity(Origin::ROOT, 0));
		assert_eq!(Instance1Module::balances(&0), 0);

		// verify then remove
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(Instance1Module::balances(&0), 0);
		assert_ok!(Instance1Module::verify_identity(Origin::ROOT, 0));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_eq!(Instance1Module::identity_infos(0).unwrap().verify_status, true);
		assert_ok!(Instance1Module::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(Instance1Module::balances(&0), 0);
	})
}

#[test]
fn multi_instances_have_independent_storage() {
	new_test_ext().execute_with(|| {
		// owners storage
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_eq!(Instance1Module::all(), vec![0]);
		assert_eq!(Instance2Module::all(), vec![]);
		// pool id storage
		assert_eq!(Instance1Module::next_pool_id(), 1);
		assert_eq!(Instance2Module::next_pool_id(), 0);

		assert_ok!(Instance2Module::create_pool(Origin::signed(ALICE)));

		// balances storage
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::balances(&0), 1000);
		assert_eq!(LiquidityCurrency::free_balance(&Instance1Module::account_id()), 1000);
		assert_eq!(Instance2Module::balances(&0), 0);
		assert_eq!(LiquidityCurrency::free_balance(&Instance2Module::account_id()), 0);
	})
}
