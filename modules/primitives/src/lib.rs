#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, Error, Input};
use sp_runtime::{traits::Convert, Permill, RuntimeDebug};
use sp_std::{prelude::*, vec};

#[macro_use]
extern crate bitmask;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub mod arithmetic;

pub use orml_prices::Price;

pub type LiquidityPoolId = u32;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	LAMI = 0,
	AUSD,
	FEUR,
	FJPY,
	FBTC,
	FETH,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct RiskThreshold {
	pub margin_call: Permill,
	pub stop_out: Permill,
}

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
		LongTwo 		= 0b0000000000000001,
		LongThree 		= 0b0000000000000010,
		LongFive 		= 0b0000000000000100,
		LongTen 		= 0b0000000000001000,
		LongTwenty 		= 0b0000000000010000,
		LongThirty		= 0b0000000000100000,
		LongFifty		= 0b0000000001000000,
		LongReserved	= 0b0000000010000000,
		ShortTwo 		= 0b0000000100000000,
		ShortThree 		= 0b0000001000000000,
		ShortFive 		= 0b0000010000000000,
		ShortTen 		= 0b0000100000000000,
		ShortTwenty		= 0b0001000000000000,
		ShortThirty		= 0b0010000000000000,
		ShortFifty		= 0b0100000000000000,
		ShortReserved	= 0b1000000000000000,
	}
}

impl Encode for Leverage {
	fn size_hint(&self) -> usize {
		1
	}

	fn encode(&self) -> Vec<u8> {
		vec![u16::trailing_zeros(**self) as u8]
	}
}

impl Decode for Leverage {
	fn decode<I: Input>(value: &mut I) -> Result<Self, Error> {
		let trailing_zeros = value.read_byte()?;
		if trailing_zeros >= 16 {
			return Err(Error::from("overflow"));
		}
		match trailing_zeros {
			0 => Ok(Leverage::LongTwo),
			1 => Ok(Leverage::LongThree),
			2 => Ok(Leverage::LongFive),
			3 => Ok(Leverage::LongTen),
			4 => Ok(Leverage::LongTwenty),
			5 => Ok(Leverage::LongThirty),
			6 => Ok(Leverage::LongFifty),
			7 => Ok(Leverage::LongReserved),
			8 => Ok(Leverage::ShortTwo),
			9 => Ok(Leverage::ShortThree),
			10 => Ok(Leverage::ShortFive),
			11 => Ok(Leverage::ShortTen),
			12 => Ok(Leverage::ShortTwenty),
			13 => Ok(Leverage::ShortThirty),
			14 => Ok(Leverage::ShortFifty),
			15 => Ok(Leverage::ShortReserved),
			_ => Err(Error::from("unknown value")),
		}
	}
}

impl Leverage {
	pub fn is_long(&self) -> bool {
		!self.is_short()
	}

	pub fn is_short(&self) -> bool {
		*self >= Leverage::ShortTwo
	}

	pub fn value(&self) -> u8 {
		match *self {
			Leverage::LongTwo | Leverage::ShortTwo => 2,
			Leverage::LongThree | Leverage::ShortThree => 3,
			Leverage::LongFive | Leverage::ShortFive => 5,
			Leverage::LongTen | Leverage::ShortTen => 10,
			Leverage::LongTwenty | Leverage::ShortTwenty => 20,
			Leverage::LongThirty | Leverage::ShortThirty => 30,
			Leverage::LongFifty | Leverage::ShortFifty => 50,
			Leverage::LongReserved | Leverage::ShortReserved => 100,
		}
	}
}

#[cfg(not(feature = "std"))]
impl core::fmt::Debug for Leverages {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Leverages {:?}", self)
	}
}

#[cfg(not(feature = "std"))]
impl core::fmt::Debug for Leverage {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "Leverage {:?}", self)
	}
}

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct SwapPeriod<Moment> {
	pub period: Moment,
	pub start: Moment,
}

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AccumulateConfig<BlockNumber> {
	pub frequency: BlockNumber,
	pub offset: BlockNumber,
}

#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPair {
	pub base: CurrencyId,
	pub quote: CurrencyId,
}

#[cfg(test)]
mod tests {
	use super::*;

	const SHORTS: [Leverage; 8] = [
		Leverage::ShortTwo,
		Leverage::ShortThree,
		Leverage::ShortFive,
		Leverage::ShortTen,
		Leverage::ShortTwenty,
		Leverage::ShortThirty,
		Leverage::ShortFifty,
		Leverage::ShortReserved,
	];

	const LONGS: [Leverage; 8] = [
		Leverage::LongTwo,
		Leverage::LongThree,
		Leverage::LongFive,
		Leverage::LongTen,
		Leverage::LongTwenty,
		Leverage::LongThirty,
		Leverage::LongFifty,
		Leverage::LongReserved,
	];

	#[test]
	fn check_leverages_all_value() {
		assert_eq!(*Leverages::all(), 0xffff);
		assert_eq!(*LONGS.iter().fold(Leverages::none(), |acc, i| (acc | *i)), 0x00ff);
		assert_eq!(*SHORTS.iter().fold(Leverages::none(), |acc, i| (acc | *i)), 0xff00);

		let mut all = LONGS.clone().to_vec();
		all.extend_from_slice(&SHORTS);

		assert_eq!(
			all.iter().fold(Leverages::none(), |acc, i| (acc | *i)),
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

	#[test]
	fn encode_decode_should_work() {
		let mut all = LONGS.clone().to_vec();
		all.extend_from_slice(&SHORTS);
		for leverage in all {
			let encoded = leverage.encode();
			let decoded = Leverage::decode(&mut &encoded[..]).unwrap();
			assert_eq!(leverage, decoded);
		}

		assert_eq!(Leverage::LongFifty, Leverage::decode(&mut &[6][..]).unwrap());

		let fifty = Leverages::from(Leverage::LongFifty | Leverage::ShortFifty);
		assert_eq!(fifty, Leverages::decode(&mut &fifty.encode()[..]).unwrap());

		let none_encoded = Leverages::none().encode();
		assert_eq!(Leverages::decode(&mut &none_encoded[..]).unwrap(), Leverages::none());

		let all_encoded = Leverages::all().encode();
		assert_eq!(Leverages::decode(&mut &all_encoded[..]).unwrap(), Leverages::all());
	}
}
