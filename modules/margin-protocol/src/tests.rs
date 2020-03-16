//! Unit tests for the margin protocol module.

#![cfg(test)]

use super::*;
use mock::*;

use core::num::NonZeroI128;
use frame_support::{assert_noop, assert_ok};
use primitives::Leverage;
use sp_runtime::PerThing;

#[test]
fn unrealized_pl_of_long_position_works() {
	ExtBuilder::default()
		// USD/JPY = 110
		.price(CurrencyId::FJPY, (1, 110))
		// EUR/JPY = 140 => EUR/USD = 140/110
		.price(CurrencyId::FEUR, (140, 110))
		.build()
		.execute_with(|| {
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: TradingPair {
					base: CurrencyId::FJPY,
					quote: CurrencyId::FEUR,
				},
				leverage: Leverage::LongTwenty,
				leveraged_held: Fixed128::from_natural(100_000),
				leveraged_debits: Fixed128::from_natural(-14_104_090),
				leveraged_held_in_usd: Fixed128::from_rational(-131_813_93, NonZeroI128::new(100).unwrap()),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				open_margin: 6_591,
			};

			assert_eq!(
				MarginProtocol::_unrealized_pl_of_position(&position),
				Ok(Fixed128::from_parts(-1073545454545441750827)),
			);
		});
}

#[test]
fn unrealized_pl_of_short_position_works() {
	ExtBuilder::default() // USD/JPY = 110
		.price(CurrencyId::FJPY, (1, 110))
		// EUR/JPY = 140 => EUR/USD = 140/110
		.price(CurrencyId::FEUR, (140, 110))
		.build()
		.execute_with(|| {
			let position: Position<Runtime> = Position {
				owner: ALICE,
				pool: MOCK_POOL,
				pair: TradingPair {
					base: CurrencyId::FJPY,
					quote: CurrencyId::FEUR,
				},
				leverage: Leverage::ShortTwenty,
				leveraged_held: Fixed128::from_natural(-100_000),
				leveraged_debits: Fixed128::from_natural(14_175_810),
				leveraged_held_in_usd: Fixed128::from_rational(133_734_06, NonZeroI128::new(100).unwrap()),
				open_accumulated_swap_rate: Fixed128::from_natural(1),
				open_margin: 6_687,
			};
			assert_eq!(
				MarginProtocol::_unrealized_pl_of_position(&position),
				Ok(Fixed128::from_parts(1470999999999987141081)),
			)
		});
}
