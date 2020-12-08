#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use traits::LiquidityPools;

fn get_free_balance(who: &AccountId) -> Balance {
	<Runtime as Config>::IdentityDepositCurrency::free_balance(who)
}

fn get_reserved_balance(who: &AccountId) -> Balance {
	<Runtime as Config>::IdentityDepositCurrency::reserved_balance(who)
}

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
		assert_eq!(Instance1Module::pools(0), Some(Pool::new(ALICE, 0)));
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
		assert_eq!(Instance1Module::liquidity(0), 1000);
		assert_ok!(Instance1Module::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(Instance1Module::pools(0), None);
		assert_eq!(Instance1Module::liquidity(0), 0);
		assert_eq!(<Instance1Module as LiquidityPools<AccountId>>::liquidity(0), 0);
	})
}

#[test]
fn should_deposit_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_eq!(Instance1Module::liquidity(0), 0);
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::liquidity(0), 1000);
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
		assert_eq!(Instance1Module::pools(0), Some(Pool::new(ALICE, 0)));
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::liquidity(0), 1000);
		assert_ok!(Instance1Module::withdraw_liquidity(Origin::signed(ALICE), 0, 500));
		assert_eq!(Instance1Module::liquidity(0), 500);
		assert_ok!(<Instance1Module as LiquidityPools<AccountId>>::withdraw_liquidity(
			&BOB, 0, 100
		));
		assert_eq!(Instance1Module::liquidity(0), 400);
	})
}

#[test]
fn should_fail_withdraw_liquidity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::liquidity(0), 1000);
		assert_eq!(
			Instance1Module::withdraw_liquidity(Origin::signed(ALICE), 0, 5000),
			Err(Error::<Runtime, Instance1>::NotEnoughBalance.into()),
		);

		assert_eq!(
			Instance1Module::withdraw_liquidity(Origin::signed(ALICE), 0, 1000),
			Err(Error::<Runtime, Instance1>::CannotWithdrawExistentialDeposit.into()),
		);

		assert_eq!(Instance1Module::liquidity(0), 1000);
	})
}

#[test]
fn should_set_identity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));

		let mut identity = IdentityInfo {
			legal_name: "laminar".as_bytes().to_vec(),
			display_name: vec![0; 201],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};
		assert_noop!(
			Instance1Module::set_identity(Origin::signed(ALICE), 0, identity.clone()),
			Error::<Runtime, Instance1>::IdentityInfoTooLong
		);

		identity.display_name = vec![];

		assert_eq!(get_free_balance(&ALICE), 100000);
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));

		identity.display_name = "Open finance platform".as_bytes().to_vec();
		assert_ok!(Instance1Module::set_identity(Origin::signed(ALICE), 0, identity));
		assert_eq!(get_free_balance(&ALICE), 99000);

		let event = mock::Event::base_liquidity_pools_Instance1(RawEvent::IdentitySet(ALICE, 0));
		assert!(System::events().iter().any(|record| record.event == event));
	})
}

#[test]
fn should_verify_identity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));

		let identity = IdentityInfo {
			legal_name: "laminar".as_bytes().to_vec(),
			display_name: vec![],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};

		assert_eq!(get_free_balance(&ALICE), 100000);
		assert_eq!(get_reserved_balance(&ALICE), 0);
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(get_free_balance(&ALICE), 99000);
		assert_eq!(get_reserved_balance(&ALICE), 1000);

		// verify
		assert_eq!(
			Instance1Module::identity_infos(0),
			Some((identity.clone(), 1000, false))
		);
		assert_ok!(Instance1Module::verify_identity(Origin::signed(UpdateOrigin::get()), 0));
		assert_eq!(Instance1Module::identity_infos(0), Some((identity.clone(), 1000, true)));
		assert_eq!(get_reserved_balance(&ALICE), 1000);
		// verify then modify
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(
			Instance1Module::identity_infos(0),
			Some((identity.clone(), 1000, false))
		);
		assert_ok!(Instance1Module::verify_identity(Origin::signed(UpdateOrigin::get()), 0));
		assert_eq!(get_reserved_balance(&ALICE), 1000);
		assert_eq!(Instance1Module::identity_infos(0), Some((identity.clone(), 1000, true)));
		assert_eq!(get_free_balance(&ALICE), 99000);

		let event = mock::Event::base_liquidity_pools_Instance1(RawEvent::IdentityVerified(0));
		assert!(System::events().iter().any(|record| record.event == event));
	})
}

#[test]
fn should_clear_identity() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));

		let identity = IdentityInfo {
			legal_name: "laminar".as_bytes().to_vec(),
			display_name: vec![],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};

		// clear without verify
		assert_eq!(get_free_balance(&ALICE), 100000);
		assert_eq!(get_reserved_balance(&ALICE), 0);
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));

		assert_eq!(get_reserved_balance(&ALICE), 1000);
		assert_eq!(
			Instance1Module::identity_infos(0),
			Some((identity.clone(), 1000, false))
		);
		assert_ok!(Instance1Module::clear_identity(Origin::signed(ALICE), 0));
		assert_eq!(get_reserved_balance(&ALICE), 0);
		assert_noop!(
			Instance1Module::clear_identity(Origin::signed(ALICE), 0),
			Error::<Runtime, Instance1>::IdentityInfoNotFound
		);
		assert_eq!(Instance1Module::identity_infos(0), None);

		// verify then clear
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(get_reserved_balance(&ALICE), 1000);
		assert_ok!(Instance1Module::verify_identity(Origin::signed(UpdateOrigin::get()), 0));
		assert_eq!(Instance1Module::identity_infos(0), Some((identity.clone(), 1000, true)));
		assert_ok!(Instance1Module::clear_identity(Origin::signed(ALICE), 0));
		assert_eq!(get_reserved_balance(&ALICE), 0);

		// verify then remove
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(get_reserved_balance(&ALICE), 1000);
		assert_ok!(Instance1Module::verify_identity(Origin::signed(UpdateOrigin::get()), 0));
		assert_eq!(Instance1Module::identity_infos(0), Some((identity.clone(), 1000, true)));
		assert_ok!(Instance1Module::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(get_reserved_balance(&ALICE), 0);
		assert_eq!(get_free_balance(&ALICE), 100000);

		let event = mock::Event::base_liquidity_pools_Instance1(RawEvent::IdentityVerified(0));
		assert!(System::events().iter().any(|record| record.event == event));
	})
}

#[test]
fn should_transfer_liquidity_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::liquidity(0), 1000);

		let identity = IdentityInfo {
			legal_name: "laminar".as_bytes().to_vec(),
			display_name: vec![],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};

		assert_eq!(get_free_balance(&ALICE), 100000);
		assert_ok!(Instance1Module::set_identity(
			Origin::signed(ALICE),
			0,
			identity.clone()
		));
		assert_eq!(get_free_balance(&ALICE), 99000);

		assert_ok!(Instance1Module::transfer_liquidity_pool(Origin::signed(ALICE), 0, BOB));
		assert_eq!(get_free_balance(&ALICE), 100000);

		let event = mock::Event::base_liquidity_pools_Instance1(RawEvent::LiquidityPoolTransferred(ALICE, 0, BOB));
		assert!(System::events().iter().any(|record| record.event == event));

		// remove pool
		assert_eq!(Instance1Module::liquidity(0), 1000);
		assert_noop!(
			Instance1Module::remove_pool(Origin::signed(ALICE), 0),
			Error::<Runtime, Instance1>::NoPermission
		);
		assert_ok!(Instance1Module::remove_pool(Origin::signed(BOB), 0),);
		assert_eq!(Instance1Module::pools(0), None);
		assert_eq!(Instance1Module::liquidity(0), 0);
		assert_eq!(<Instance1Module as LiquidityPools<AccountId>>::liquidity(0), 0);
		assert_eq!(LiquidityCurrency::free_balance(&ALICE), 99000);
		assert_eq!(LiquidityCurrency::free_balance(&BOB), 101000);
	})
}

#[test]
fn multi_instances_have_independent_storage() {
	new_test_ext().execute_with(|| {
		// owners storage
		assert_ok!(Instance1Module::create_pool(Origin::signed(ALICE)));
		let event = mock::Event::base_liquidity_pools_Instance1(RawEvent::LiquidityPoolCreated(ALICE, 0));
		assert!(System::events().iter().any(|record| record.event == event));

		assert_eq!(Instance1Module::all(), vec![0]);
		assert_eq!(Instance2Module::all(), Vec::<u32>::new());
		// pool id storage
		assert_eq!(Instance1Module::next_pool_id(), 1);
		assert_eq!(Instance2Module::next_pool_id(), 0);

		assert_ok!(Instance2Module::create_pool(Origin::signed(ALICE)));
		let event = mock::Event::base_liquidity_pools_Instance2(RawEvent::LiquidityPoolCreated(ALICE, 0));
		assert!(System::events().iter().any(|record| record.event == event));

		// balances storage
		assert_ok!(Instance1Module::deposit_liquidity(Origin::signed(ALICE), 0, 1000));
		assert_eq!(Instance1Module::liquidity(0), 1000);
		assert_eq!(LiquidityCurrency::free_balance(&Instance1Module::account_id()), 1000);
		assert_eq!(Instance2Module::liquidity(0), 0);
		assert_eq!(LiquidityCurrency::free_balance(&Instance2Module::account_id()), 0);
	})
}
