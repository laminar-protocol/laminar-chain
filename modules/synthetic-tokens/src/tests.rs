//! Unit tests for the synthetic-tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{alice, ExtBuilder, SyntheticTokens, System, TestEvent, FEUR, ROOT};
use sp_runtime::Permill;

macro_rules! assert_noop_root {
	($x:expr) => {
		assert_noop!($x, frame_system::Error::RequireRootOrigin.into());
	};
}

#[test]
fn root_set_extreme_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SyntheticTokens::extreme_ratio(FEUR), None);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_extreme_ratio(ROOT, FEUR, ratio));
		assert_eq!(SyntheticTokens::extreme_ratio(FEUR), Some(ratio));

		let event = TestEvent::synthetic_tokens(RawEvent::ExtremeRatioUpdated(FEUR, ratio));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn non_root_set_extreme_ratio_fails() {
	ExtBuilder::default().build().execute_with(|| {
		let ratio = Permill::from_percent(1);
		assert_noop_root!(SyntheticTokens::set_extreme_ratio(alice(), FEUR, ratio));
	});
}

#[test]
fn root_set_liquidation_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SyntheticTokens::liquidation_ratio(FEUR), None);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_liquidation_ratio(ROOT, FEUR, ratio));
		assert_eq!(SyntheticTokens::liquidation_ratio(FEUR), Some(ratio));

		let event = TestEvent::synthetic_tokens(RawEvent::LiquidationRatioUpdated(FEUR, ratio));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn non_root_set_liquidation_ratio_fails() {
	ExtBuilder::default().build().execute_with(|| {
		let ratio = Permill::from_percent(1);
		assert_noop_root!(SyntheticTokens::set_liquidation_ratio(alice(), FEUR, ratio));
	});
}

#[test]
fn root_set_collateral_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SyntheticTokens::collateral_ratio(FEUR), None);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_collateral_ratio(ROOT, FEUR, ratio));
		assert_eq!(SyntheticTokens::collateral_ratio(FEUR), Some(ratio));

		let event = TestEvent::synthetic_tokens(RawEvent::CollateralRatioUpdated(FEUR, ratio));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn non_root_set_collateral_ratio_fails() {
	ExtBuilder::default().build().execute_with(|| {
		let ratio = Permill::from_percent(1);
		assert_noop_root!(SyntheticTokens::set_collateral_ratio(alice(), FEUR, ratio));
	});
}

#[test]
fn liquidation_ratio_or_default() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			SyntheticTokens::liquidation_ratio_or_default(FEUR),
			LIQUIDATION_RATIO_DEFAULT
		);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_liquidation_ratio(ROOT, FEUR, ratio));
		assert_eq!(SyntheticTokens::liquidation_ratio_or_default(FEUR), ratio);
	});
}

#[test]
fn extreme_ratio_or_default() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(SyntheticTokens::extreme_ratio_or_default(FEUR), EXTREME_RATIO_DEFAULT);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_extreme_ratio(ROOT, FEUR, ratio));
		assert_eq!(SyntheticTokens::extreme_ratio_or_default(FEUR), ratio);
	});
}

#[test]
fn collateral_ratio_or_default() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			SyntheticTokens::collateral_ratio_or_default(FEUR),
			COLLATERAL_RATIO_DEFAULT
		);

		let ratio = Permill::from_percent(1);
		assert_ok!(SyntheticTokens::set_collateral_ratio(ROOT, FEUR, ratio));
		assert_eq!(SyntheticTokens::collateral_ratio_or_default(FEUR), ratio);
	});
}

#[test]
fn no_incentive_if_collateral_less_than_synthetic_value() {
	ExtBuilder::default().build().execute_with(|| {
		let ratio = FixedU128::from_rational(1, 2);
		assert_eq!(SyntheticTokens::incentive_ratio(FEUR, ratio), FixedU128::from_parts(0));
	});
}

fn plus_one(ratio: FixedU128) -> FixedU128 {
	ratio.saturating_add(FixedU128::from_rational(1, 1))
}

#[test]
fn no_incentive_if_equal_or_above_liquidation_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		// equal
		assert_eq!(
			SyntheticTokens::incentive_ratio(FEUR, plus_one(LIQUIDATION_RATIO_DEFAULT.into())),
			FixedU128::from_parts(0)
		);

		// above
		let ratio = FixedU128::from_rational(11, 100);
		assert!(ratio > SyntheticTokens::liquidation_ratio_or_default(FEUR).into());
		assert_eq!(
			SyntheticTokens::incentive_ratio(FEUR, plus_one(ratio)),
			FixedU128::from_parts(0)
		);
	});
}

#[test]
fn full_incentive_if_equal_or_below_extreme_ratio() {
	ExtBuilder::default().build().execute_with(|| {
		// equal
		assert_eq!(
			SyntheticTokens::incentive_ratio(FEUR, plus_one(EXTREME_RATIO_DEFAULT.into())),
			FixedU128::from_rational(1, 1)
		);

		// below
		let ratio = FixedU128::from_parts(0);
		assert!(ratio < SyntheticTokens::extreme_ratio_or_default(FEUR).into());
		assert_eq!(
			SyntheticTokens::incentive_ratio(FEUR, plus_one(ratio)),
			FixedU128::from_rational(1, 1)
		);
	});
}

// Given: ratio 10%, extreme 0%, liquidation 100%.
// Incentive should be 90%.
#[test]
fn proportional_incentive_between_extreme_and_liquidation() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(SyntheticTokens::set_extreme_ratio(ROOT, FEUR, Permill::zero()));
		assert_ok!(SyntheticTokens::set_liquidation_ratio(ROOT, FEUR, Permill::one()));

		let ten_percent = FixedU128::from_rational(1, 10);
		assert_eq!(
			SyntheticTokens::incentive_ratio(FEUR, plus_one(ten_percent)),
			FixedU128::from_rational(9, 10)
		);
	});
}
