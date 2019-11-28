#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sr_primitives::RuntimeDebug;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub type LiquidityPoolId = u32;

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	FLOW = 0,
	AUSD,
	FEUR,
	FJPY,
}
