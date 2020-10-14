#![cfg_attr(not(feature = "std"), no_std)]
// Suppress warning generated from bitmask! macro.
#![allow(clippy::transmute_ptr_to_ptr)]

use codec::{Decode, Encode, Error, Input};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	FixedU128, MultiSignature, RuntimeDebug,
};

use sp_arithmetic::FixedI128;
use sp_std::{prelude::*, vec};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[macro_use]
extern crate bitmask;

pub mod arithmetic;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on
/// the chain.
pub type Signature = MultiSignature;

/// Alias to the public key used for this chain, actually a `MultiSigner`. Like
/// the signature, this also isn't a fixed size when encoded, as different
/// cryptos have different size public keys.
pub type AccountPublic = <Signature as Verify>::Signer;

/// Alias to the opaque account ID type for this chain, actually a
/// `AccountId32`. This is always 32 bytes.
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of
/// them.
pub type AccountIndex = u32;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An instant or duration in time.
pub type Moment = u64;

/// Counter for the number of eras that have passed.
pub type EraIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Signed version of Balance
pub type Amount = i128;

pub type AuctionId = u32;

pub type Share = u128;

/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Opaque, encoded, unchecked extrinsic.
pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

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
	FAUD,
	FCAD,
	FCHF,
	FXAU,
	FOIL,
}

pub type Price = FixedU128;

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

/// Swap accumulation configuration.
///
/// Swap would be accumulated every `frequency` time and on `now % offset == 0`.
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AccumulateConfig<Moment> {
	/// Accumulation frequency.
	pub frequency: Moment,

	/// Accumulation time offset.
	pub offset: Moment,
}

/// Trading pair.
#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPair {
	/// The base currency.
	pub base: CurrencyId,

	/// The quote currency.
	pub quote: CurrencyId,
}

/// Liquidity pool identity info.
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default, Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct IdentityInfo {
	/// Legal name.
	///
	/// Legal name may be business entity name, or owner's personal legal_name name.
	pub legal_name: Vec<u8>,

	/// Display name.
	pub display_name: Vec<u8>,

	/// Website URL.
	pub web: Vec<u8>,

	/// Email.
	pub email: Vec<u8>,

	/// Image URL.
	pub image_url: Vec<u8>,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct SwapRate {
	pub long: FixedI128,
	pub short: FixedI128,
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum DataProviderId {
	Aggregated = 0,
	Laminar = 1,
	Band = 2,
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
