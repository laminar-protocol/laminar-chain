//! Runtime API definition for margin protocol module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use module_primitives::LiquidityPoolId;
use orml_utilities::Fixed128;
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default)]
pub struct TraderInfo {
	pub equity: Fixed128,
	pub margin_held: Fixed128,
	pub margin_level: Fixed128,
	pub free_margin: Fixed128,
	pub unrealized_pl: Fixed128,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default)]
pub struct PoolInfo {
	pub enp: Fixed128,
	pub ell: Fixed128,
	pub required_deposit: Fixed128,
}

sp_api::decl_runtime_apis! {
	pub trait MarginProtocolApi<AccountId> where
		AccountId: Codec,
	{
		fn trader_info(who: AccountId, pool_id: LiquidityPoolId) -> TraderInfo;
		fn pool_info(pool_id: LiquidityPoolId) -> Option<PoolInfo>;
	}
}
