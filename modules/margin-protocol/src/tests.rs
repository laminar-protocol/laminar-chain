//! Unit tests for the margin protocol module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{ExtBuilder, MarginProtocol, Origin, Runtime, ALICE};

#[test]
fn trader_margin_call_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let risk_threshold = RiskThreshold {
			margin_call: Permill::from_percent(5),
			stop_out: Permill::from_percent(3),
		};
		<MarginProtocol as crate::Store>::TraderRiskThreshold::put(risk_threshold);
		assert_ok!(MarginProtocol::trader_margin_call(Origin::ROOT, 0));
		assert_ok!(MarginProtocol::trader_margin_call(Origin::signed(ALICE), 0));
	});
}

#[test]
fn trader_become_safe_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let risk_threshold = RiskThreshold {
			margin_call: Permill::from_percent(5),
			stop_out: Permill::from_percent(3),
		};
		<MarginProtocol as crate::Store>::TraderRiskThreshold::put(risk_threshold);
		assert_ok!(MarginProtocol::trader_become_safe(Origin::ROOT, 0));
	});
}

#[test]
fn trader_liquidate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let risk_threshold = RiskThreshold {
			margin_call: Permill::from_percent(5),
			stop_out: Permill::from_percent(3),
		};
		<MarginProtocol as crate::Store>::TraderRiskThreshold::put(risk_threshold);
		assert_ok!(MarginProtocol::trader_liquidate(Origin::ROOT, 0));
	});
}
