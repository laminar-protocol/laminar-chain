//! Unit tests for the synthetic-protocol module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	alice, greedy_slippage, tolerable_slippage, CurrencyId, ExtBuilder, SyntheticProtocol, SyntheticTokens, System,
	TestEvent, DEFAULT_PRICE, MOCK_POOL, MOCK_PRICE_SOURCE,
};

#[test]
fn mint_fails_if_balance_too_low() {
	ExtBuilder::default().build_and_reset_env().execute_with(|| {
		assert_noop!(
			SyntheticProtocol::mint(alice(), MOCK_POOL, CurrencyId::FEUR, 1, tolerable_slippage()),
			Error::BalanceTooLow.into()
		);
	});
}

#[test]
fn mint_fails_if_no_price() {
	ExtBuilder::default()
		.one_hundred_usd_for_alice()
		.build_and_reset_env()
		.execute_with(|| {
			unsafe {
				MOCK_PRICE_SOURCE.set_none();
			}

			assert_noop!(
				SyntheticProtocol::mint(alice(), MOCK_POOL, CurrencyId::FEUR, 1, tolerable_slippage()),
				Error::NoPrice.into()
			);
		});
}

// TODO: fix this test
//#[test]
//fn mint_fails_if_slippage_too_greedy() {
//	ExtBuilder::default()
//		.one_hundred_usd_for_alice()
//		.build_and_reset_env()
//		.execute_with(|| {
//			assert_noop!(
//				SyntheticProtocol::mint(alice(), MOCK_POOL, CurrencyId::FEUR, 1, greedy_slippage()),
//				Error::SlippageTooHigh.into()
//			);
//		});
//}
