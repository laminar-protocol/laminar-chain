//! Unit tests for the synthetic-protocol module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	origin_of, AccountId, Balance, CollateralCurrency, CurrencyId, ExtBuilder, MockLiquidityPools, MockPrices,
	SyntheticCurrency, SyntheticProtocol, SyntheticTokens, System, TestEvent, ALICE, BOB, MOCK_POOL, ONE_MILL,
};

fn mint_feur(who: AccountId, amount: Balance) -> result::Result<(), &'static str> {
	SyntheticProtocol::mint(
		origin_of(who),
		MOCK_POOL,
		CurrencyId::FEUR,
		amount,
		Permill::from_percent(10),
	)
}

fn redeem_ausd(who: AccountId, amount: Balance) -> result::Result<(), &'static str> {
	SyntheticProtocol::redeem(
		origin_of(who),
		MOCK_POOL,
		CurrencyId::FEUR,
		amount,
		Permill::from_percent(10),
	)
}

fn mock_pool_balance() -> Balance {
	MockLiquidityPools::balance(MOCK_POOL)
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
					Permill::from_rational_approximation(9u32, 1_000u32)
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
fn mint_does_correct_math() {
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
			// = 330_033.0033 ~ 330_033
			let synthetic = 330_033;

			// total collateral
			// = synthetic * price * ( 1 + additional_collateral_ratio)
			// = 330_033 * 3 * (1 + 0.1)
			// = 1_089_108.9 ~ 1_089_109
			let total_collateralized = 1_089_109;

			// collateral from liquidity pool
			// = total_collateral - ONE_MILL
			// = 89_108.9 ~ 89_109
			let collateral_from_pool = 89_109;

			// alice collateralized, synthetic minted
			assert_eq!(collateral_balance(&ALICE), 0);
			assert_eq!(synthetic_balance(&ALICE), synthetic);
			assert_eq!(SyntheticCurrency::total_issuance(), synthetic);

			// liquidity pool collateralized
			assert_eq!(mock_pool_balance(), ONE_MILL - collateral_from_pool);

			// collateral locked in synthetic-tokens module account
			assert_eq!(collateral_balance(&SyntheticTokens::account_id()), total_collateralized);

			// position added
			assert_eq!(position(), (total_collateralized, synthetic));

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

#[test]
fn redeem_fails_if_not_enough_synthetic() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			assert_noop!(redeem_ausd(ALICE, ONE_MILL + 1), Error::BalanceTooLow.into());
		});
}

#[test]
fn redeem_fails_if_no_price() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			MockPrices::set_mock_price(CurrencyId::FEUR, None);

			assert_noop!(redeem_ausd(ALICE, 1), Error::NoPrice.into());
		});
}

#[test]
fn redeem_fails_if_slippage_too_greedy() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			assert_noop!(
				SyntheticProtocol::redeem(
					origin_of(ALICE),
					MOCK_POOL,
					CurrencyId::FEUR,
					1,
					Permill::from_rational_approximation(9u32, 1_000u32)
				),
				Error::SlippageTooHigh.into()
			);
		});
}

#[test]
fn redeem_fails_if_synthetic_position_too_low() {
	let another_pool = 101;
	assert_ne!(another_pool, MOCK_POOL);

	ExtBuilder::default()
		.balances(vec![ALICE, MOCK_POOL, another_pool], ONE_MILL)
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL / 10));

			// mint via another pool
			assert_ok!(SyntheticProtocol::mint(
				origin_of(ALICE),
				another_pool,
				CurrencyId::FEUR,
				ONE_MILL / 10,
				Permill::from_percent(10),
			));

			// redeem all in one pool, synthetic position would be too low
			assert_noop!(
				redeem_ausd(ALICE, SyntheticCurrency::balance(&ALICE)),
				Error::LiquidityPoolSyntheticPositionTooLow.into()
			);
		});
}

#[test]
fn redeem_fails_if_collateral_position_too_low() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			// price changed, collateral position would be too low to redeem
			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::from_rational(4, 1)));

			assert_noop!(
				redeem_ausd(ALICE, SyntheticCurrency::balance(&ALICE)),
				Error::LiquidityPoolCollateralPositionTooLow.into()
			);
		});
}

#[test]
fn redeem_fails_if_not_enough_locked_collateral() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			// mock not enough locked collateral
			assert_ok!(CollateralCurrency::withdraw(
				&SyntheticTokens::account_id(),
				ONE_MILL / 2
			));

			assert_noop!(
				redeem_ausd(ALICE, SyntheticCurrency::balance(&ALICE)),
				Error::NotEnoughLockedCollateralAvailable.into(),
			);
		});
}

#[test]
fn redeem_does_correct_math() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			// after minting...
			// minted_synthetic = 330_033
			// collateral_position = 1_089_109
			// collateral_from_pool = 89_109

			let collateral_from_pool = 89_109;
			let total_collateralized = 1_089_109;

			let synthetic_to_redeem = 100_000;
			assert_ok!(redeem_ausd(ALICE, synthetic_to_redeem));

			// rest_of_synthetic
			// = minted_synthetic - synthetic_to_redeem
			// = 330_033 - 100_000
			let rest_of_synthetic = 230_033;

			// redeemed_collateral
			// = synthetic * bid_price
			// = 100_000 * 3 * (1 - 0.01)
			// = 297_000
			let redeemed_collateral = 297_000;

			// required_collateral
			// = new_synthetic_value * (1 + additional_collateral_ratio)
			// = 230_033 * 3 * (1 + 0.1)
			// = 759_108.9 ~ 759_109

			let new_collateral_position = 759_109;

			// pool_refund_collateral
			// = collateral_position_delta - redeemed_collateral
			// = 1_089_109 - 759_109 - 297_000
			// = 33_000
			let pool_refund_collateral = 33_000;

			// alice redeemed collateral, synthetic burned
			assert_eq!(collateral_balance(&ALICE), redeemed_collateral);
			assert_eq!(synthetic_balance(&ALICE), rest_of_synthetic);
			assert_eq!(SyntheticCurrency::total_issuance(), rest_of_synthetic);

			// liquidity pool got collateral refund
			assert_eq!(
				mock_pool_balance(),
				ONE_MILL - collateral_from_pool + pool_refund_collateral
			);

			// locked collateral in synthetic-tokens module account got released
			assert_eq!(
				collateral_balance(&SyntheticTokens::account_id()),
				total_collateralized - redeemed_collateral - pool_refund_collateral
			);

			// position update
			assert_eq!(position(), (new_collateral_position, rest_of_synthetic));
		});
}

#[test]
fn pool_makes_profit() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			assert_ok!(redeem_ausd(ALICE, synthetic_balance(&ALICE)));
			assert!(mock_pool_balance() > ONE_MILL);
		});
}

#[test]
fn buyer_could_take_profit() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL / 10));
			// wow price rose
			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::from_rational(31, 10)));

			assert_ok!(redeem_ausd(ALICE, synthetic_balance(&ALICE)));
			assert!(collateral_balance(&ALICE) > ONE_MILL);
		});
}

#[test]
fn buyer_could_stop_loss() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL / 10));
			// ops price dropped
			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::from_rational(29, 10)));

			assert_ok!(redeem_ausd(ALICE, synthetic_balance(&ALICE)));
			assert!(collateral_balance(&ALICE) < ONE_MILL);
		});
}

#[test]
fn mint_and_redeem_by_multi_buyers() {
	ExtBuilder::default()
		.balances(vec![ALICE, BOB, MOCK_POOL], ONE_MILL)
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL / 10));
			assert_ok!(mint_feur(BOB, ONE_MILL / 15));

			assert_ne!(collateral_balance(&ALICE), collateral_balance(&BOB));
			assert_ne!(synthetic_balance(&ALICE), synthetic_balance(&BOB));
			assert_eq!(
				SyntheticCurrency::total_issuance(),
				synthetic_balance(&ALICE) + synthetic_balance(&BOB)
			);

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::from_rational(29, 10)));

			assert_ok!(redeem_ausd(ALICE, synthetic_balance(&ALICE)));
			assert_ok!(redeem_ausd(BOB, synthetic_balance(&BOB)));
			assert_eq!(SyntheticCurrency::total_issuance(), 0);
		});
}
