#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{traits::Convert, RuntimeDebug};

#[macro_use]
extern crate bitmask;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub use orml_prices::Price;

pub type LiquidityPoolId = u32;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	LAMI = 0,
	AUSD,
	FEUR,
	FJPY,
}

// TODO: set the actual accuracy
const BALANCE_ACCURACY: u128 = 1_000_000_000_000_000_000;

pub type Balance = u128;

pub struct BalancePriceConverter;

impl Convert<Balance, Price> for BalancePriceConverter {
	fn convert(balance: Balance) -> Price {
		// if same accuracy, use `from_parts` to get best performance
		if BALANCE_ACCURACY == Price::accuracy() {
			return Price::from_parts(balance);
		}

		Price::from_rational(balance, BALANCE_ACCURACY)
	}
}

impl Convert<Price, Balance> for BalancePriceConverter {
	fn convert(price: Price) -> Balance {
		let deconstructed = price.deconstruct();
		let price_accuracy = Price::accuracy();

		if BALANCE_ACCURACY == price_accuracy {
			deconstructed
		} else if price_accuracy > BALANCE_ACCURACY {
			// could never overflow, as `price_accuracy / BALANCE_ACCURACY > 1`
			deconstructed / (price_accuracy / BALANCE_ACCURACY)
		} else {
			// could never overflow in real world case, but if it did, there's nothing to be done
			// other than saturating
			deconstructed.saturating_mul(BALANCE_ACCURACY / price_accuracy)
		}
	}
}

bitmask! {
	#[derive(Encode, Decode, Default)]
	pub mask Leverages: u16 where flags Leverage {
		LongTwo 	= 0b0000000000000001,
		LongThree 	= 0b0000000000000010,
		LongFive 	= 0b0000000000000100,
		LongTen 	= 0b0000000000001000,
		LongTwenty 	= 0b0000000000010000,
		LongThirty	= 0b0000000000100000,
		LongFifty	= 0b0000000001000000,
		ShortTwo 	= 0b0000000010000000,
		ShortThree 	= 0b0000000100000000,
		ShortFive 	= 0b0000001000000000,
		ShortTen 	= 0b0000010000000000,
		ShortTwenty	= 0b0000100000000000,
		ShortThirty	= 0b0001000000000000,
		ShortFifty	= 0b0010000000000000,
	}
}

impl Leverage {
	#[allow(dead_code)]
	fn is_long(&self) -> bool {
		!self.is_short()
	}

	#[allow(dead_code)]
	fn is_short(&self) -> bool {
		*self >= Leverage::ShortTwo
	}

	#[allow(dead_code)]
	fn value(&self) -> u8 {
		match *self {
			Leverage::LongTwo | Leverage::ShortTwo => 2,
			Leverage::LongThree | Leverage::ShortThree => 3,
			Leverage::LongFive | Leverage::ShortFive => 5,
			Leverage::LongTen | Leverage::ShortTen => 10,
			Leverage::LongTwenty | Leverage::ShortTwenty => 20,
			Leverage::LongThirty | Leverage::ShortThirty => 30,
			Leverage::LongFifty | Leverage::ShortFifty => 50,
		}
	}
}

#[cfg(not(feature = "std"))]
impl core::fmt::Debug for Leverages {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Leverages {:?}", self)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const SHORTS: [Leverage; 7] = [
		Leverage::ShortTwo,
		Leverage::ShortThree,
		Leverage::ShortFive,
		Leverage::ShortTen,
		Leverage::ShortTwenty,
		Leverage::ShortThirty,
		Leverage::ShortFifty,
	];

	const LONGS: [Leverage; 7] = [
		Leverage::LongTwo,
		Leverage::LongThree,
		Leverage::LongFive,
		Leverage::LongTen,
		Leverage::LongTwenty,
		Leverage::LongThirty,
		Leverage::LongFifty,
	];

	#[test]
	fn check_leverages_all_value() {
		assert_eq!(*Leverages::all(), 0b11111111111111);
		assert_eq!(
			*LONGS.iter().fold(Leverages::none(), |acc, i| (acc | *i)),
			0b00000001111111
		);
		assert_eq!(
			*SHORTS.iter().fold(Leverages::none(), |acc, i| (acc | *i)),
			0b11111110000000
		);

		let mut merged = LONGS.clone().to_vec();
		merged.append(&mut SHORTS.clone().to_vec());

		assert_eq!(
			merged.iter().fold(Leverages::none(), |acc, i| (acc | *i)),
			Leverages::all()
		);
	}

	#[test]
	fn long_short_should_work() {
		for leverage in SHORTS.iter() {
			assert_eq!(leverage.is_short(), true);
			assert_eq!(leverage.is_long(), false);
		}

		for leverage in LONGS.iter() {
			assert_eq!(leverage.is_short(), false);
			assert_eq!(leverage.is_long(), true);
		}
	}

	#[test]
	fn value_should_work() {
		assert_eq!(Leverage::LongTwo.value(), 2);
		assert_eq!(Leverage::LongThree.value(), 3);
		assert_eq!(Leverage::LongFive.value(), 5);
		assert_eq!(Leverage::LongTen.value(), 10);
		assert_eq!(Leverage::LongTwenty.value(), 20);
		assert_eq!(Leverage::LongThirty.value(), 30);
		assert_eq!(Leverage::LongFifty.value(), 50);
		assert_eq!(Leverage::ShortTwo.value(), 2);
		assert_eq!(Leverage::ShortThree.value(), 3);
		assert_eq!(Leverage::ShortFive.value(), 5);
		assert_eq!(Leverage::ShortTen.value(), 10);
		assert_eq!(Leverage::ShortTwenty.value(), 20);
		assert_eq!(Leverage::ShortThirty.value(), 30);
		assert_eq!(Leverage::ShortFifty.value(), 50);
	}
}
