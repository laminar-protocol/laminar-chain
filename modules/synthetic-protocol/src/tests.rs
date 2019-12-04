//! Unit tests for the synthetic-protocol module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	alice, CurrencyId, ExtBuilder, MockLiquidityPools, MockPrices, SyntheticProtocol, SyntheticTokens, System,
	TestEvent, MOCK_POOL,
};

#[test]
fn mint_fails_if_balance_too_low() {
	ExtBuilder::default().synthetic_price_three().build().execute_with(|| {
		assert_noop!(
			SyntheticProtocol::mint(alice(), MOCK_POOL, CurrencyId::FEUR, 1, Permill::from_percent(1)),
			Error::BalanceTooLow.into()
		);
	});
}

#[test]
fn mint_fails_if_no_price() {
	ExtBuilder::default()
		.one_hundred_usd_for_alice()
		.build()
		.execute_with(|| {
			assert_noop!(
				SyntheticProtocol::mint(alice(), MOCK_POOL, CurrencyId::FEUR, 1, Permill::from_percent(1)),
				Error::NoPrice.into()
			);
		});
}

#[test]
fn mint_fails_if_slippage_too_greedy() {
	ExtBuilder::default()
		.one_hundred_usd_for_alice()
		.synthetic_price_three()
		.point_five_percent_spread()
		.build()
		.execute_with(|| {
			assert_noop!(
				SyntheticProtocol::mint(
					alice(),
					MOCK_POOL,
					CurrencyId::FEUR,
					1,
					Permill::from_rational_approximation(4u32, 1000u32)
				),
				Error::SlippageTooHigh.into()
			);
		});
}
