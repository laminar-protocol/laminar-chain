//! Unit tests for the synthetic-tokens module.

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_noop, assert_ok};
use sp_runtime::{
	traits::{BadOrigin, Saturating},
	Permill,
};

#[allow(unused_macros)]
macro_rules! assert_noop_root {
	($x:expr) => {
		assert_noop!($x, BadOrigin);
	};
}

#[test]
fn root_set_extreme_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SyntheticTokens::ratios(CurrencyId::FEUR).extreme, None);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_extreme_ratio(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::FEUR,
			ratio
		));
		assert_eq!(SyntheticTokens::ratios(CurrencyId::FEUR).extreme, Some(ratio));

		let event = TestEvent::synthetic_tokens(Event::ExtremeRatioUpdated(CurrencyId::FEUR, ratio));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn non_root_set_extreme_ratio_fails() {
	ExtBuilder::default().build().execute_with(|| {
		let ratio = Permill::from_percent(1);

		assert_noop!(
			SyntheticTokens::set_extreme_ratio(bob(), CurrencyId::FEUR, ratio),
			BadOrigin
		);

		assert_ok!(SyntheticTokens::set_extreme_ratio(alice(), CurrencyId::FEUR, ratio));
	});
}

#[test]
fn root_set_liquidation_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SyntheticTokens::ratios(CurrencyId::FEUR).liquidation, None);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_liquidation_ratio(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::FEUR,
			ratio
		));
		assert_eq!(SyntheticTokens::ratios(CurrencyId::FEUR).liquidation, Some(ratio));

		let event = TestEvent::synthetic_tokens(Event::LiquidationRatioUpdated(CurrencyId::FEUR, ratio));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn non_root_set_liquidation_ratio_fails() {
	ExtBuilder::default().build().execute_with(|| {
		let ratio = Permill::from_percent(1);
		assert_noop!(
			SyntheticTokens::set_liquidation_ratio(bob(), CurrencyId::FEUR, ratio),
			BadOrigin
		);

		assert_ok!(SyntheticTokens::set_liquidation_ratio(alice(), CurrencyId::FEUR, ratio));
	});
}

#[test]
fn root_set_collateral_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SyntheticTokens::ratios(CurrencyId::FEUR).collateral, None);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_collateral_ratio(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::FEUR,
			ratio
		));
		assert_eq!(SyntheticTokens::ratios(CurrencyId::FEUR).collateral, Some(ratio));

		let event = TestEvent::synthetic_tokens(Event::CollateralRatioUpdated(CurrencyId::FEUR, ratio));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn non_root_set_collateral_ratio_fails() {
	ExtBuilder::default().build().execute_with(|| {
		let ratio = Permill::from_percent(1);
		assert_noop!(
			SyntheticTokens::set_collateral_ratio(bob(), CurrencyId::FEUR, ratio),
			BadOrigin
		);

		assert_ok!(SyntheticTokens::set_collateral_ratio(alice(), CurrencyId::FEUR, ratio));
	});
}

#[test]
fn liquidation_ratio_or_default() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			SyntheticTokens::liquidation_ratio_or_default(CurrencyId::FEUR),
			<Runtime as Config>::DefaultLiquidationRatio::get()
		);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_liquidation_ratio(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::FEUR,
			ratio
		));
		assert_eq!(SyntheticTokens::liquidation_ratio_or_default(CurrencyId::FEUR), ratio);
	});
}

#[test]
fn extreme_ratio_or_default() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			SyntheticTokens::extreme_ratio_or_default(CurrencyId::FEUR),
			<Runtime as Config>::DefaultExtremeRatio::get()
		);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_extreme_ratio(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::FEUR,
			ratio
		));
		assert_eq!(SyntheticTokens::extreme_ratio_or_default(CurrencyId::FEUR), ratio);
	});
}

#[test]
fn collateral_ratio_or_default() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			SyntheticTokens::collateral_ratio_or_default(CurrencyId::FEUR),
			<Runtime as Config>::DefaultCollateralRatio::get()
		);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_collateral_ratio(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::FEUR,
			ratio
		));
		assert_eq!(SyntheticTokens::collateral_ratio_or_default(CurrencyId::FEUR), ratio);
	});
}

#[test]
fn no_incentive_if_collateral_less_than_synthetic_value() {
	ExtBuilder::default().build().execute_with(|| {
		let ratio = FixedU128::saturating_from_rational(1, 2);
		assert_eq!(
			SyntheticTokens::incentive_ratio(CurrencyId::FEUR, ratio),
			FixedU128::from_inner(0)
		);
	});
}

fn plus_one(ratio: FixedU128) -> FixedU128 {
	ratio.saturating_add(FixedU128::saturating_from_rational(1, 1))
}

#[test]
fn no_incentive_if_equal_or_above_liquidation_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		// equal
		assert_eq!(
			SyntheticTokens::incentive_ratio(
				CurrencyId::FEUR,
				plus_one(<Runtime as Config>::DefaultLiquidationRatio::get().into())
			),
			FixedU128::from_inner(0)
		);

		// above
		let ratio = FixedU128::saturating_from_rational(11, 100);
		assert!(ratio > SyntheticTokens::liquidation_ratio_or_default(CurrencyId::FEUR).into());
		assert_eq!(
			SyntheticTokens::incentive_ratio(CurrencyId::FEUR, plus_one(ratio)),
			FixedU128::from_inner(0)
		);
	});
}

#[test]
fn full_incentive_if_equal_or_below_extreme_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		// equal
		assert_eq!(
			SyntheticTokens::incentive_ratio(
				CurrencyId::FEUR,
				plus_one(<Runtime as Config>::DefaultExtremeRatio::get().into())
			),
			FixedU128::saturating_from_rational(1, 1)
		);

		// below
		let ratio = FixedU128::from_inner(0);
		assert!(ratio < SyntheticTokens::extreme_ratio_or_default(CurrencyId::FEUR).into());
		assert_eq!(
			SyntheticTokens::incentive_ratio(CurrencyId::FEUR, plus_one(ratio)),
			FixedU128::saturating_from_rational(1, 1)
		);
	});
}

// Given: ratio 10%, extreme 0%, liquidation 100%.
// Incentive should be 90%.
#[test]
fn proportional_incentive_between_extreme_and_liquidation() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(SyntheticTokens::set_extreme_ratio(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::FEUR,
			Permill::zero()
		));
		assert_ok!(SyntheticTokens::set_liquidation_ratio(
			Origin::signed(UpdateOrigin::get()),
			CurrencyId::FEUR,
			Permill::one()
		));

		let ten_percent = FixedU128::saturating_from_rational(1, 10);
		assert_eq!(
			SyntheticTokens::incentive_ratio(CurrencyId::FEUR, plus_one(ten_percent)),
			FixedU128::saturating_from_rational(9, 10)
		);
	});
}

#[test]
fn should_add_remove_get_position() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SyntheticTokens::positions(0, CurrencyId::FEUR), Position::default());
		assert_eq!(SyntheticTokens::get_position(0, CurrencyId::FEUR), (0, 0));

		SyntheticTokens::add_position(0, CurrencyId::FEUR, 1, 2);

		assert_eq!(
			SyntheticTokens::positions(0, CurrencyId::FEUR),
			Position {
				collateral: 1,
				synthetic: 2
			}
		);
		assert_eq!(SyntheticTokens::get_position(0, CurrencyId::FEUR), (1, 2));

		SyntheticTokens::remove_position(0, CurrencyId::FEUR, 1, 1);

		assert_eq!(
			SyntheticTokens::positions(0, CurrencyId::FEUR),
			Position {
				collateral: 0,
				synthetic: 1
			}
		);
		assert_eq!(SyntheticTokens::get_position(0, CurrencyId::FEUR), (0, 1));

		SyntheticTokens::remove_position(0, CurrencyId::FEUR, 1, 1);

		assert_eq!(SyntheticTokens::positions(0, CurrencyId::FEUR), Position::default());
		assert_eq!(SyntheticTokens::get_position(0, CurrencyId::FEUR), (0, 0));
	});
}
