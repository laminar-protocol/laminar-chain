//! Runtime API definition for synthetic protocol module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use module_primitives::{CurrencyId, LiquidityPoolId};
use sp_arithmetic::FixedU128;
use sp_core::RuntimeDebug;
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct SyntheticPoolState {
	pub collateral_ratio: FixedU128,
	pub is_safe: bool,
}

sp_api::decl_runtime_apis! {
	pub trait SyntheticProtocolApi<AccountId> where
		AccountId: Codec,
	{
		fn pool_state(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<SyntheticPoolState>;
	}
}
