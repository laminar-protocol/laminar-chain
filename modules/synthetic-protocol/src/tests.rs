//! Unit tests for the synthetic-protocol module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use sp_runtime::{DispatchResult, Permill};

fn mint_feur(who: AccountId, amount: Balance) -> DispatchResult {
	SyntheticProtocol::mint(
		origin_of(who),
		MOCK_POOL,
		CurrencyId::FEUR,
		amount,
		Price::saturating_from_rational(4, 1),
	)
}

fn redeem_ausd(who: AccountId, amount: Balance) -> DispatchResult {
	SyntheticProtocol::redeem(
		origin_of(who),
		MOCK_POOL,
		CurrencyId::FEUR,
		amount,
		Price::saturating_from_rational(2, 1),
	)
}

fn liquidate(who: AccountId, amount: Balance) -> DispatchResult {
	SyntheticProtocol::liquidate(origin_of(who), MOCK_POOL, CurrencyId::FEUR, amount)
}

fn add_collateral(who: AccountId, amount: Balance) -> DispatchResult {
	SyntheticProtocol::add_collateral(origin_of(who), MOCK_POOL, CurrencyId::FEUR, amount)
}

fn withdraw_collateral(who: AccountId) -> DispatchResult {
	SyntheticProtocol::withdraw_collateral(origin_of(who), MOCK_POOL, CurrencyId::FEUR)
}

fn mock_pool_liquidity() -> Balance {
	MockLiquidityPools::liquidity(MOCK_POOL)
}

fn collateral_balance(who: AccountId) -> Balance {
	CollateralCurrency::free_balance(&who)
}

fn synthetic_balance(who: AccountId) -> Balance {
	SyntheticCurrency::free_balance(&who)
}

fn position() -> (Balance, Balance) {
	TestSyntheticTokens::get_position(MOCK_POOL, CurrencyId::FEUR)
}

fn set_mock_feur_price(x: u128, y: u128) {
	MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(x, y)));
}

fn set_mock_feur_price_none() {
	MockPrices::set_mock_price(CurrencyId::FEUR, None);
}

#[test]
fn mint_fails_if_balance_too_low() {
	ExtBuilder::default()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(mint_feur(ALICE, 100), orml_tokens::Error::<Runtime>::BalanceTooLow);
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
			assert_noop!(mint_feur(ALICE, 100), Error::<Runtime>::NoPrice);
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
					Price::saturating_from_rational(3, 1),
				),
				Error::<Runtime>::AskPriceTooHigh
			);
		});
}

#[test]
fn mint_fails_if_wrong_spread_ratio_config() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.spread(Price::from_fraction(0.02))
		.additional_collateral_ratio(Permill::from_percent(1))
		.build()
		.execute_with(|| {
			assert_noop!(
				mint_feur(ALICE, 100),
				Error::<Runtime>::NegativeAdditionalCollateralAmount
			);
		});
}

#[test]
fn mint_fails_if_pool_has_no_liquidity() {
	ExtBuilder::default()
		.balances(vec![(ALICE, CurrencyId::AUSD, ONE_MILL)])
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(
				mint_feur(ALICE, ONE_MILL),
				Error::<Runtime>::InsufficientLiquidityInPool,
			);
		});
}

#[test]
fn mint_fails_if_currency_is_not_supported() {
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
					CurrencyId::LAMI,
					ONE_MILL,
					Price::saturating_from_rational(3, 1),
				),
				Error::<Runtime>::NotValidSyntheticCurrencyId
			);
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
			assert_eq!(collateral_balance(ALICE), 0);
			assert_eq!(synthetic_balance(ALICE), synthetic);
			assert_eq!(SyntheticCurrency::total_issuance(), synthetic);

			// liquidity pool collateralized
			assert_eq!(mock_pool_liquidity(), ONE_MILL - collateral_from_pool);

			// collateral locked in synthetic-tokens module account
			assert_eq!(
				collateral_balance(TestSyntheticTokens::account_id()),
				total_collateralized
			);

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
			assert_noop!(
				redeem_ausd(ALICE, ONE_MILL + 1),
				orml_tokens::Error::<Runtime>::BalanceTooLow
			);
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

			set_mock_feur_price_none();

			assert_noop!(redeem_ausd(ALICE, 1), Error::<Runtime>::NoPrice);
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
					Price::saturating_from_rational(3, 1),
				),
				Error::<Runtime>::BidPriceTooLow
			);
		});
}

#[test]
fn redeem_fails_if_synthetic_position_too_low() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, CurrencyId::AUSD, ONE_MILL),
			(MOCK_POOL, CurrencyId::AUSD, ONE_MILL),
			(ANOTHER_MOCK_POOL, CurrencyId::AUSD, ONE_MILL),
		])
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL / 10));

			// mint via another pool
			assert_ok!(SyntheticProtocol::mint(
				origin_of(ALICE),
				ANOTHER_MOCK_POOL,
				CurrencyId::FEUR,
				ONE_MILL / 10,
				Price::saturating_from_rational(4, 1),
			));

			// redeem all in one pool, synthetic position would be too low
			assert_noop!(
				redeem_ausd(ALICE, SyntheticCurrency::free_balance(&ALICE)),
				Error::<Runtime>::InsufficientSyntheticInPosition
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
			set_mock_feur_price(4, 1);

			assert_noop!(
				redeem_ausd(ALICE, SyntheticCurrency::free_balance(&ALICE)),
				Error::<Runtime>::InsufficientCollateralInPosition
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
				&TestSyntheticTokens::account_id(),
				ONE_MILL / 2
			));

			assert_noop!(
				redeem_ausd(ALICE, SyntheticCurrency::free_balance(&ALICE)),
				Error::<Runtime>::InsufficientLockedCollateral,
			);
		});
}

#[test]
fn redeem_fails_if_currency_is_not_supported() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(
				SyntheticProtocol::redeem(
					origin_of(ALICE),
					MOCK_POOL,
					CurrencyId::LAMI,
					ONE_MILL,
					Price::saturating_from_rational(3, 1),
				),
				Error::<Runtime>::NotValidSyntheticCurrencyId,
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
			assert_eq!(collateral_balance(ALICE), redeemed_collateral);
			assert_eq!(synthetic_balance(ALICE), rest_of_synthetic);
			assert_eq!(SyntheticCurrency::total_issuance(), rest_of_synthetic);

			// liquidity pool got collateral refund
			assert_eq!(
				mock_pool_liquidity(),
				ONE_MILL - collateral_from_pool + pool_refund_collateral
			);

			// locked collateral in synthetic-tokens module account got released
			assert_eq!(
				collateral_balance(TestSyntheticTokens::account_id()),
				total_collateralized - redeemed_collateral - pool_refund_collateral
			);

			// position update
			assert_eq!(position(), (new_collateral_position, rest_of_synthetic));

			// event deposited
			let event = TestEvent::synthetic_protocol(RawEvent::Redeemed(
				ALICE,
				CurrencyId::FEUR,
				MOCK_POOL,
				redeemed_collateral,
				synthetic_to_redeem,
			));
			assert!(System::events().iter().any(|record| record.event == event));
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
			assert_ok!(redeem_ausd(ALICE, synthetic_balance(ALICE)));
			assert!(mock_pool_liquidity() > ONE_MILL);
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
			set_mock_feur_price(31, 10);

			assert_ok!(redeem_ausd(ALICE, synthetic_balance(ALICE)));
			assert!(collateral_balance(ALICE) > ONE_MILL);
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
			set_mock_feur_price(29, 10);

			assert_ok!(redeem_ausd(ALICE, synthetic_balance(ALICE)));
			assert!(collateral_balance(ALICE) < ONE_MILL);
		});
}

#[test]
fn mint_and_redeem_by_multi_buyers() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, CurrencyId::AUSD, ONE_MILL),
			(BOB, CurrencyId::AUSD, ONE_MILL),
			(MOCK_POOL, CurrencyId::AUSD, ONE_MILL),
		])
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL / 10));
			assert_ok!(mint_feur(BOB, ONE_MILL / 15));

			assert_ne!(collateral_balance(ALICE), collateral_balance(BOB));
			assert_ne!(synthetic_balance(ALICE), synthetic_balance(BOB));
			assert_eq!(
				SyntheticCurrency::total_issuance(),
				synthetic_balance(ALICE) + synthetic_balance(BOB)
			);

			set_mock_feur_price(29, 10);

			assert_ok!(redeem_ausd(ALICE, synthetic_balance(ALICE)));
			assert_ok!(redeem_ausd(BOB, synthetic_balance(BOB)));
			assert_eq!(SyntheticCurrency::total_issuance(), 0);
		});
}

#[test]
fn liquidate_fails_if_liquidator_not_enough_synthetic() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			set_mock_feur_price(32, 10);

			assert_noop!(liquidate(BOB, 1), orml_tokens::Error::<Runtime>::BalanceTooLow);
		});
}

#[test]
fn liquidate_fails_if_no_price() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			set_mock_feur_price_none();

			assert_noop!(liquidate(ALICE, 1), Error::<Runtime>::NoPrice);
		});
}

#[test]
fn liquidate_fails_if_synthetic_position_too_low() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, CurrencyId::AUSD, ONE_MILL),
			(MOCK_POOL, CurrencyId::AUSD, ONE_MILL),
			(ANOTHER_MOCK_POOL, CurrencyId::AUSD, ONE_MILL),
		])
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL / 2));
			assert_ok!(SyntheticProtocol::mint(
				origin_of(ALICE),
				ANOTHER_MOCK_POOL,
				CurrencyId::FEUR,
				ONE_MILL / 2,
				Price::saturating_from_rational(4, 1),
			));

			set_mock_feur_price(32, 10);
			assert_noop!(
				liquidate(ALICE, synthetic_balance(ALICE)),
				Error::<Runtime>::InsufficientSyntheticInPosition
			);
		});
}

#[test]
fn liquidate_fails_if_collateral_position_too_low() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			set_mock_feur_price(4, 1);

			assert_noop!(
				liquidate(ALICE, synthetic_balance(ALICE)),
				Error::<Runtime>::InsufficientCollateralInPosition
			);
		});
}

#[test]
fn liquidate_fails_if_still_in_safe_position() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			set_mock_feur_price(31, 10);
			assert_noop!(liquidate(ALICE, 1), Error::<Runtime>::StillInSafePosition);
		});
}

#[test]
fn liquidate_fails_if_currency_is_not_supported() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(
				SyntheticProtocol::liquidate(origin_of(ALICE), MOCK_POOL, CurrencyId::LAMI, ONE_MILL,),
				Error::<Runtime>::NotValidSyntheticCurrencyId,
			);
		});
}

#[test]
fn liquidate_does_correct_math() {
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

			let minted_synthetic = 330_033;
			let collateral_from_pool = 89_109;
			let total_collateralized = 1_089_109;

			set_mock_feur_price(32, 10);

			// mock Bob has synthetic
			let burned_synthetic = 100_000;
			assert_ok!(SyntheticCurrency::deposit(&BOB, burned_synthetic));
			// let Bob to liquidate, to make math verification easier
			assert_ok!(liquidate(BOB, burned_synthetic));

			// liquidized_collateral
			// = burned_synthetic * bid_price
			// = 100_000 * 3.2 * (1 - 0.01)
			// = 316_800
			let liquidized_collateral = 316_800;

			// new_collateral_position
			// = current_collateral_position - liquidized_collateral
			// = 1_089_109 - 316_800
			// = 772_309

			// synthetic_position_value
			// = synthetic_position * price
			// = (330_033 * 3.2)
			// = 1_056_105.6 ~ 1_056_105 (FixedU128 type got floored int)

			// current_ratio
			// = collateral_position / synthetic_position_value
			// = 1_089_109 / 1_056_105
			// = 1.031250680566799702

			// with_current_ratio
			// = new_synthetic_position_value * current_ratio
			// = (synthetic_position - burned_synthetic) * price * current_ratio
			// = (330_033 - 100_000) * 3.2 * 1.031250680566799702
			// = 736_105.6 * 1.031250680566799702
			// ~= 736_105 * 1.031250680566799702
			// = 759_108.782219 ~ 759_108

			// incentive_ratio
			// = (liquidation_ratio - ratio) / (liquidation_ratio - extreme_ratio)
			// = (0.05 - 0.031250680566799702) / (0.05 - 0.01)
			// = 0.46873298583

			// available_for_incentive
			// = new_collateral_position - with_current_ratio
			// = 772_309 - 759_108
			// = 13_201
			let available_for_incentive = 13_201;

			// incentive
			// = available_for_incentive * incentive_ratio
			// = 13_201 * 0.46873298583
			// = 6_187.74414594 ~ 6_187
			let incentive = 6_187;

			// pool_refund_collateral
			// = available_for_incentive - incentive
			// = 13_201 - 6_187
			// = 7_014
			let pool_refund_collateral = 7_014;

			// Bob liquidized and got incentive collateral
			assert_eq!(synthetic_balance(BOB), 0);
			assert_eq!(collateral_balance(BOB), liquidized_collateral + incentive);
			assert_eq!(SyntheticCurrency::total_issuance(), minted_synthetic);

			// liquidity pool got refund
			assert_eq!(
				mock_pool_liquidity(),
				ONE_MILL - collateral_from_pool + pool_refund_collateral
			);

			// locked collateral in synthetic-tokens module account got released
			let collateral_position_delta = liquidized_collateral + available_for_incentive;
			assert_eq!(
				collateral_balance(TestSyntheticTokens::account_id()),
				total_collateralized - collateral_position_delta
			);

			// position updated
			assert_eq!(
				position(),
				(
					total_collateralized - collateral_position_delta,
					minted_synthetic - burned_synthetic
				)
			);

			// event deposited
			let event = TestEvent::synthetic_protocol(RawEvent::Liquidated(
				BOB,
				CurrencyId::FEUR,
				MOCK_POOL,
				liquidized_collateral,
				burned_synthetic,
			));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn add_collateral_fails_if_balance_too_low() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			assert_noop!(add_collateral(ALICE, 1), orml_tokens::Error::<Runtime>::BalanceTooLow);
		});
}

#[test]
fn add_collateral_fails_if_currency_is_not_supported() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(
				SyntheticProtocol::add_collateral(origin_of(ALICE), MOCK_POOL, CurrencyId::LAMI, ONE_MILL),
				Error::<Runtime>::NotValidSyntheticCurrencyId
			);
		});
}

#[test]
fn add_collateral_works() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL / 2));
			let minted_synthetic_amount = SyntheticCurrency::total_issuance();
			let (collateral_position, synthetic_position) = position();
			let pool_balance = mock_pool_liquidity();

			let added_collateral = 1_000;
			assert_ok!(add_collateral(ALICE, added_collateral));

			assert_eq!(collateral_balance(ALICE), ONE_MILL / 2 - added_collateral);

			// minted synthetic amount stays the same
			assert_eq!(SyntheticCurrency::total_issuance(), minted_synthetic_amount);

			// liquidity pool balance stays the same
			assert_eq!(mock_pool_liquidity(), pool_balance);

			// position change matched
			let (new_collateral_position, new_synthetic_position) = position();
			assert_eq!(new_synthetic_position, synthetic_position);
			assert_eq!(new_collateral_position, collateral_position + added_collateral);

			// event deposited
			let event = TestEvent::synthetic_protocol(RawEvent::CollateralAdded(
				ALICE,
				CurrencyId::FEUR,
				MOCK_POOL,
				added_collateral,
			));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn only_owner_could_withdraw_collateral() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			assert_noop!(withdraw_collateral(BOB), Error::<Runtime>::NoPermission);
		});
}

#[test]
fn withdraw_collateral_fails_if_no_price() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			MockPrices::set_mock_price(CurrencyId::FEUR, None);
			assert_noop!(withdraw_collateral(ALICE), Error::<Runtime>::NoPrice);
		});
}

#[test]
fn withdraw_collateral_fails_if_not_enough_locked_collateral() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			let (collateral_position, _) = position();

			// mock not enough locked collateral
			assert_ok!(CollateralCurrency::withdraw(
				&TestSyntheticTokens::account_id(),
				collateral_position
			));

			set_mock_feur_price(2, 1);

			assert_noop!(
				withdraw_collateral(ALICE),
				Error::<Runtime>::InsufficientLockedCollateral
			);
		});
}

#[test]
fn withdraw_collateral_fails_if_currency_is_not_supported() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_noop!(
				SyntheticProtocol::withdraw_collateral(origin_of(ALICE), MOCK_POOL, CurrencyId::LAMI),
				Error::<Runtime>::NotValidSyntheticCurrencyId
			);
		});
}

#[test]
fn withdraw_collateral_does_correct_math() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));
			let minted_synthetic_amount = SyntheticCurrency::total_issuance();
			let (collateral_position, synthetic_position) = position();
			let pool_balance = mock_pool_liquidity();

			// after minted...
			// minted_synthetic = 330_033
			// collateral_position = 1_089_109
			// collateral_from_pool = 89_109

			MockPrices::set_mock_price(CurrencyId::FEUR, Some(Price::saturating_from_rational(29, 10)));

			// required_collateral
			// = new_synthetic_value * (1 + additional_collateral_ratio)
			// = 330_033 * 2.9 * (1 + 0.1)
			// = 957_095.7 * 1.1
			// ~= 957_095 * 1.1 (FixedU128 type got floored int)
			// ~= 1_052_804

			// collateral_position_delta
			// = collateral_position - required_collateral
			// = 1_089_109 - 1_052_804
			// = 36_305
			let withdrew_amount = 36_305;

			assert_ok!(withdraw_collateral(ALICE));

			// ALICE withdrew collateral
			assert_eq!(collateral_balance(ALICE), withdrew_amount);

			// minted synthetic amount stays the same
			assert_eq!(SyntheticCurrency::total_issuance(), minted_synthetic_amount);

			// liquidity pool balance stays the same
			assert_eq!(mock_pool_liquidity(), pool_balance);

			// collateral position changed
			let (new_collateral_position, new_synthetic_position) = position();
			assert_eq!(new_collateral_position, collateral_position - withdrew_amount);
			assert_eq!(new_synthetic_position, synthetic_position);

			// event deposited
			let event = TestEvent::synthetic_protocol(RawEvent::CollateralWithdrew(
				ALICE,
				CurrencyId::FEUR,
				MOCK_POOL,
				withdrew_amount,
			));
			assert!(System::events().iter().any(|record| record.event == event));
		});
}

#[test]
fn mint_fails_if_not_allowed() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.set_is_allowed(false)
		.build()
		.execute_with(|| {
			assert_noop!(mint_feur(ALICE, 100), Error::<Runtime>::CannotMintInPool);
		});
}

#[test]
fn can_redeem_with_not_allowed_position() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, 1000));

			MockLiquidityPools::set_is_allowed(false);

			assert_noop!(mint_feur(ALICE, 100), Error::<Runtime>::CannotMintInPool);

			assert_ok!(redeem_ausd(ALICE, 100));
		});
}

#[test]
fn can_liquidate_with_not_allowed_position() {
	ExtBuilder::default()
		.one_million_for_alice_n_mock_pool()
		.synthetic_price_three()
		.one_percent_spread()
		.ten_percent_additional_collateral_ratio()
		.build()
		.execute_with(|| {
			assert_ok!(mint_feur(ALICE, ONE_MILL));

			MockLiquidityPools::set_is_allowed(false);

			set_mock_feur_price(32, 10);

			let burned_synthetic = 100_000;
			assert_ok!(SyntheticCurrency::deposit(&BOB, burned_synthetic));

			assert_ok!(liquidate(BOB, burned_synthetic));
		});
}

#[test]
fn mint_all_of_collateral() {
	ExtBuilder::default()
		.balances(vec![
			(ALICE, CurrencyId::AUSD, 1000),
			(MOCK_POOL, CurrencyId::AUSD, 1000),
		])
		.synthetic_price(Price::saturating_from_rational(1, 1))
		.one_percent_spread()
		.additional_collateral_ratio(Permill::from_percent(100))
		.build()
		.execute_with(|| {
			assert_eq!(mock_pool_liquidity(), 1000);
			assert_eq!(collateral_balance(ALICE), 1000);
			assert_eq!(synthetic_balance(ALICE), 0);

			assert_ok!(mint_feur(ALICE, 1000));
			assert_eq!(collateral_balance(ALICE), 0);
			assert_eq!(synthetic_balance(ALICE), 990);
			assert_eq!(mock_pool_liquidity(), 20);
		});
}
