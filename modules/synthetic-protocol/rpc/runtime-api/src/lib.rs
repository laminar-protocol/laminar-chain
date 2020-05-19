//! Runtime API definition for synthetic protocol module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use module_primitives::{CurrencyId, LiquidityPoolId};
use orml_utilities::FixedU128;
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default, Debug)]
pub struct PoolInfo {
	pub collateral_ratio: FixedU128,
	pub is_safe: bool,
}

sp_api::decl_runtime_apis! {
	pub trait SyntheticProtocolApi<AccountId> where
		AccountId: Codec,
	{
		fn pool_info(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<PoolInfo>;
	}
}
