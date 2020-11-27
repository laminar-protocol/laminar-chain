#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use sp_runtime::Permill;

use primitives::CurrencyId;
use traits::{LiquidityPools, SyntheticProtocolLiquidityPools};

#[test]
fn should_disable_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			Default::default()
		);
		assert_ok!(ModuleLiquidityPools::set_synthetic_enabled(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			true,
		));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			SyntheticPoolCurrencyOption {
				bid_spread: None,
				ask_spread: None,
				additional_collateral_ratio: None,
				synthetic_enabled: true,
			},
		);
		assert_ok!(BaseLiquidityPools::disable_pool(Origin::signed(ALICE), 0));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			Default::default()
		);
	})
}

#[test]
fn should_remove_pool() {
	new_test_ext().execute_with(|| {
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_ok!(ModuleLiquidityPools::deposit_liquidity(&ALICE, 0, 1000));
		assert_eq!(BaseLiquidityPools::liquidity(0), 1000);
		assert_ok!(BaseLiquidityPools::remove_pool(Origin::signed(ALICE), 0));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			Default::default()
		);
		assert_eq!(BaseLiquidityPools::owner(0), None);
		assert_eq!(BaseLiquidityPools::liquidity(0), 0);
		assert_eq!(<ModuleLiquidityPools as LiquidityPools<AccountId>>::liquidity(0), 0);
	})
}

#[test]
fn should_set_spread() {
	new_test_ext().execute_with(|| {
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(BaseLiquidityPools::owner(0), Some(ALICE));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			Default::default()
		);
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Price::from_inner(80),
			Price::from_inner(60)
		));

		let pool_option = SyntheticPoolCurrencyOption {
			bid_spread: Some(Price::from_inner(80)),
			ask_spread: Some(Price::from_inner(60)),
			additional_collateral_ratio: None,
			synthetic_enabled: false,
		};

		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			pool_option
		);

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::bid_spread(0, CurrencyId::AUSD),
			Some(Price::from_inner(80))
		);
		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::ask_spread(0, CurrencyId::AUSD),
			Some(Price::from_inner(60))
		);
	})
}

#[test]
fn should_set_max_spread() {
	new_test_ext().execute_with(|| {
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(BaseLiquidityPools::owner(0), Some(ALICE));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			Default::default()
		);
		// no max spread
		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Price::from_inner(100),
			Price::from_inner(100)
		));

		// set max spread to 30%
		assert_ok!(ModuleLiquidityPools::set_max_spread(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::AUSD,
			Price::from_inner(30)
		));

		assert_noop!(
			ModuleLiquidityPools::set_spread(
				Origin::signed(ALICE),
				0,
				CurrencyId::AUSD,
				Price::from_inner(32),
				Price::from_inner(28)
			),
			Error::<Runtime>::SpreadTooHigh
		);

		assert_ok!(ModuleLiquidityPools::set_spread(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Price::from_inner(28),
			Price::from_inner(29)
		));

		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			SyntheticPoolCurrencyOption {
				bid_spread: Some(Price::from_inner(28)),
				ask_spread: Some(Price::from_inner(29)),
				additional_collateral_ratio: None,
				synthetic_enabled: false,
			},
		);
	});
}

#[test]
fn should_set_additional_collateral_ratio() {
	new_test_ext().execute_with(|| {
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(BaseLiquidityPools::owner(0), Some(ALICE));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			Default::default()
		);
		assert_ok!(ModuleLiquidityPools::set_min_additional_collateral_ratio(
			Origin::signed(UpdateOrigin::get()),
			Permill::from_percent(120)
		));
		assert_ok!(ModuleLiquidityPools::set_additional_collateral_ratio(
			Origin::signed(ALICE),
			0,
			CurrencyId::AUSD,
			Some(Permill::from_percent(120)),
		));

		let pool_option = SyntheticPoolCurrencyOption {
			bid_spread: None,
			ask_spread: None,
			additional_collateral_ratio: Some(Permill::from_percent(120)),
			synthetic_enabled: false,
		};

		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			pool_option
		);

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(120)
		);
		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::additional_collateral_ratio(
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
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(BaseLiquidityPools::owner(0), Some(ALICE));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			Default::default()
		);
		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::additional_collateral_ratio(
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
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::additional_collateral_ratio(
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
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(120),
		);

		assert_ok!(ModuleLiquidityPools::set_min_additional_collateral_ratio(
			Origin::signed(UpdateOrigin::get()),
			Permill::from_percent(150)
		));

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(150)
		);

		assert_ok!(ModuleLiquidityPools::set_min_additional_collateral_ratio(
			Origin::signed(UpdateOrigin::get()),
			Permill::from_percent(100)
		));

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::additional_collateral_ratio(
				0,
				CurrencyId::AUSD
			),
			Permill::from_percent(120)
		);

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::additional_collateral_ratio(
				0,
				CurrencyId::FJPY
			),
			Permill::from_percent(100)
		);
	})
}

#[test]
fn should_set_synthetic_enabled() {
	new_test_ext().execute_with(|| {
		assert_ok!(BaseLiquidityPools::create_pool(Origin::signed(ALICE)));
		assert_eq!(BaseLiquidityPools::owner(0), Some(ALICE));
		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			Default::default()
		);
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

		let pool_option = SyntheticPoolCurrencyOption {
			bid_spread: None,
			ask_spread: None,
			additional_collateral_ratio: None,
			synthetic_enabled: true,
		};

		assert_eq!(
			ModuleLiquidityPools::pool_currency_options(0, CurrencyId::AUSD),
			pool_option
		);

		assert_eq!(
			<ModuleLiquidityPools as SyntheticProtocolLiquidityPools<AccountId>>::can_mint(0, CurrencyId::AUSD),
			true
		);
	});
}
