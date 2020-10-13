//! Runtime API definition for margin protocol module.

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]

use codec::{Codec, Decode, Encode};
use laminar_primitives::LiquidityPoolId;
use sp_arithmetic::FixedI128;
use sp_core::RuntimeDebug;
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct MarginTraderState {
	pub equity: FixedI128,
	pub margin_held: FixedI128,
	pub margin_level: FixedI128,
	pub free_margin: FixedI128,
	pub unrealized_pl: FixedI128,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default, RuntimeDebug)]
pub struct MarginPoolState {
	pub enp: FixedI128,
	pub ell: FixedI128,
	pub required_deposit: FixedI128,
}

sp_api::decl_runtime_apis! {
	pub trait MarginProtocolApi<AccountId> where
		AccountId: Codec,
	{
		fn trader_state(who: AccountId, pool_id: LiquidityPoolId) -> MarginTraderState;
		fn pool_state(pool_id: LiquidityPoolId) -> Option<MarginPoolState>;
	}
}
