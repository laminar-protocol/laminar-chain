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
		LongOne 	= 0b0000000000000001,
		LongTwo 	= 0b0000000000000010,
		LongThree 	= 0b0000000000000100,
		LongFive 	= 0b0000000000001000,
		LongTen 	= 0b0000000000010000,
		LongTwenty 	= 0b0000000000100000,
		LongThirty	= 0b0000000001000000,
		LongFifty	= 0b0000000010000000,
		ShortOne 	= 0b0000000100000000,
		ShortTwo 	= 0b0000001000000000,
		ShortThree 	= 0b0000010000000000,
		ShortFive 	= 0b0000100000000000,
		ShortTen 	= 0b0001000000000000,
		ShortTwenty = 0b0010000000000000,
		ShortThirty	= 0b0100000000000000,
		ShortFifty	= 0b1000000000000000,
	}
}

impl Leverage {
	#[allow(dead_code)]
	fn is_long(&self) -> bool {
		!self.is_short()
	}

	#[allow(dead_code)]
	fn is_short(&self) -> bool {
		*self >= Leverage::ShortOne
	}

	#[allow(dead_code)]
	fn value(&self) -> u8 {
		let long_val = if self.is_long() { **self } else { ((**self) >> 8) };
		match long_val {
			0b0000000000000001 => 1,
			0b0000000000000010 => 2,
			0b0000000000000100 => 3,
			0b0000000000001000 => 5,
			0b0000000000010000 => 10,
			0b0000000000100000 => 20,
			0b0000000001000000 => 30,
			0b0000000010000000 => 50,
			_ => 0, // should never happen
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

	#[test]
	fn is_long_should_work() {
		assert_eq!(Leverage::LongOne.is_long(), true);
		assert_eq!(Leverage::LongTwo.is_long(), true);
		assert_eq!(Leverage::LongThree.is_long(), true);
		assert_eq!(Leverage::LongFive.is_long(), true);
		assert_eq!(Leverage::LongTen.is_long(), true);
		assert_eq!(Leverage::LongTwenty.is_long(), true);
		assert_eq!(Leverage::LongThirty.is_long(), true);
		assert_eq!(Leverage::LongFifty.is_long(), true);
		assert_eq!(Leverage::ShortOne.is_long(), false);
		assert_eq!(Leverage::ShortTwo.is_long(), false);
		assert_eq!(Leverage::ShortThree.is_long(), false);
		assert_eq!(Leverage::ShortFive.is_long(), false);
		assert_eq!(Leverage::ShortTen.is_long(), false);
		assert_eq!(Leverage::ShortTwenty.is_long(), false);
		assert_eq!(Leverage::ShortThirty.is_long(), false);
		assert_eq!(Leverage::ShortFifty.is_long(), false);
	}

	#[test]
	fn is_short_should_work() {
		assert_eq!(Leverage::LongOne.is_short(), false);
		assert_eq!(Leverage::LongTwo.is_short(), false);
		assert_eq!(Leverage::LongThree.is_short(), false);
		assert_eq!(Leverage::LongFive.is_short(), false);
		assert_eq!(Leverage::LongTen.is_short(), false);
		assert_eq!(Leverage::LongTwenty.is_short(), false);
		assert_eq!(Leverage::LongThirty.is_short(), false);
		assert_eq!(Leverage::LongFifty.is_short(), false);
		assert_eq!(Leverage::ShortOne.is_short(), true);
		assert_eq!(Leverage::ShortTwo.is_short(), true);
		assert_eq!(Leverage::ShortThree.is_short(), true);
		assert_eq!(Leverage::ShortFive.is_short(), true);
		assert_eq!(Leverage::ShortTen.is_short(), true);
		assert_eq!(Leverage::ShortTwenty.is_short(), true);
		assert_eq!(Leverage::ShortThirty.is_short(), true);
		assert_eq!(Leverage::ShortFifty.is_short(), true);
	}

	#[test]
	fn value_should_work() {
		assert_eq!(Leverage::LongOne.value(), 1);
		assert_eq!(Leverage::LongTwo.value(), 2);
		assert_eq!(Leverage::LongThree.value(), 3);
		assert_eq!(Leverage::LongFive.value(), 5);
		assert_eq!(Leverage::LongTen.value(), 10);
		assert_eq!(Leverage::LongTwenty.value(), 20);
		assert_eq!(Leverage::LongThirty.value(), 30);
		assert_eq!(Leverage::LongFifty.value(), 50);
		assert_eq!(Leverage::ShortOne.value(), 1);
		assert_eq!(Leverage::ShortTwo.value(), 2);
		assert_eq!(Leverage::ShortThree.value(), 3);
		assert_eq!(Leverage::ShortFive.value(), 5);
		assert_eq!(Leverage::ShortTen.value(), 10);
		assert_eq!(Leverage::ShortTwenty.value(), 20);
		assert_eq!(Leverage::ShortThirty.value(), 30);
		assert_eq!(Leverage::ShortFifty.value(), 50);
	}
}
