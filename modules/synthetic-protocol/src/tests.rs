//! Unit tests for the synthetic-protocol module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	origin_of, AccountId, Balance, CollateralCurrency, CurrencyId, ExtBuilder, MockPrices,
	SyntheticCurrency, SyntheticProtocol, SyntheticTokens, System, TestEvent, ALICE, MOCK_POOL, ONE_MILL,
};

fn mint_feur(who: AccountId, amount: Balance) -> result::Result<(), &'static str> {
	SyntheticProtocol::mint(
		origin_of(who),
		MOCK_POOL,
		CurrencyId::FEUR,
		amount,
		Permill::from_percent(2),
	)
}

fn collateral_balance(who: &AccountId) -> Balance {
	CollateralCurrency::balance(who)
}

fn synthetic_balance(who: &AccountId) -> Balance {
	SyntheticCurrency::balance(who)
}

fn position() -> (Balance, Balance) {
	SyntheticTokens::get_position(MOCK_POOL, CurrencyId::FEUR)
}

#[test]
fn mint_fails_if_balance_too_low() {
	ExtBuilder::default()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(mint_feur(ALICE, 1), Error::BalanceTooLow.into());
		});
}

#[test]
fn mint_fails_if_no_price() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(mint_feur(ALICE, 1), Error::NoPrice.into());
		});
}

#[test]
fn mint_fails_if_slippage_too_greedy() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(
				SyntheticProtocol::mint(
					origin_of(ALICE),
					MOCK_POOL,
					CurrencyId::FEUR,
					1,
					Permill::from_rational_approximation(9u32, 1000u32)
				),
				Error::SlippageTooHigh.into()
			);
		});
}

#[test]
fn mint_fails_if_wrong_spread_ratio_config() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.additional_collateral_ratio(Permill::from_percent(1))
		.build()
		.execute_with(|| {
			assert_noop!(mint_feur(ALICE, 1), Error::NegativeAdditionalCollateralAmount.into());
		});
}

#[test]
fn mint_fails_if_pool_has_no_liquidity() {
	ExtBuilder::default()
		.balances(vec![ALICE], ONE_MILL)
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(mint_feur(ALICE, ONE_MILL), Error::LiquidityProviderBalanceTooLow.into(),);
		});
}

#[test]
fn mint() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			// minted synthetic
			// = ONE_MILL / ask_price
			// = ONE_MILL / (3 * (1 + 0.01))
			// = 330033.0033
			let synthetic = 330033;

			// total collateral
			// = synthetic * price * ( 1 + additional_collateral_ratio)
			// = 330033 * 3 * (1 + 0.1)
			// = 1089108.9
			let total_collateral = 1089109;

			// collateral from liquidity pool
			// = total_collateral - ONE_MILL
			// = 89108.9
			let collateral_from_pool = 89109;

			// alice collateralized, synthetic minted
			assert_eq!(collateral_balance(&ALICE), 0);
			assert_eq!(synthetic_balance(&ALICE), synthetic);
			assert_eq!(SyntheticCurrency::total_issuance(), synthetic);

			// liquidity pool collateralized
			assert_eq!(collateral_balance(&MOCK_POOL), ONE_MILL - collateral_from_pool);

			// collateral locked in synthetic-tokens module account
			assert_eq!(
				CollateralCurrency::balance(&SyntheticTokens::account_id()),
				total_collateral
			);

			// position added
			assert_eq!(position(), (total_collateral, synthetic));

			// event emitted
			let event = TestEvent::synthetic_protocol(RawEvent::Minted(
				ALICE,
				CurrencyId::FEUR,
				MOCK_POOL,
				ONE_MILL,
				synthetic,
			));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}
