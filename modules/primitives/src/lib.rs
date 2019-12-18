#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{traits::Convert, RuntimeDebug};

#[macro_use]
extern crate bitmask;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub use orml_prices::Price;

pub type LiquidityPoolId = u32;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug)]
// `PartialOrd` and `Ord` are only used for tests
#[cfg_attr(feature = "std", derive(PartialOrd, Ord, Serialize, Deserialize))]
pub enum CurrencyId {
	FLOW = 0,
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
		ShortFive 	= 0b0000000000000001,
		LongFive 	= 0b0000000000000010,
		ShortTen 	= 0b0000000000000100,
		LongTen 	= 0b0000000000001000,
		ShortTwenty	= 0b0000000000010000,
		LongTwenty 	= 0b0000000000100000,
		ShortThirty	= 0b0000000001000000,
		LongThirty	= 0b0000000010000000,
		ShortForty	= 0b0000000100000000,
		LongForty	= 0b0000001000000000,
		ShortFifty	= 0b0000010000000000,
		LongFifty	= 0b0000100000000000,
	}
}

impl Leverage {
	fn is_long(self) -> bool {
		!self.is_short()
	}

	fn is_short(self) -> bool {
		(Leverage::ShortFive
			| Leverage::ShortTen
			| Leverage::ShortTwenty
			| Leverage::ShortThirty
			| Leverage::ShortForty
			| Leverage::ShortFifty)
			.contains(self)
	}
}

#[test]
fn should_check_short_and_long() {
	assert_eq!(Leverage::ShortFifty.is_short(), true);
	assert_eq!(Leverage::LongFifty.is_short(), false);
	assert_eq!(Leverage::LongFifty.is_long(), true);
	assert_eq!(Leverage::ShortFive.is_long(), false);
}
