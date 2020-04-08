//! Runtime API definition for margin protocol module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Codec, Decode, Encode};
use sp_std::prelude::*;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default)]
pub struct TraderInfo<Fixed128> {
	pub equity: Fixed128,
	pub margin_balances: Vec<Fixed128>,
	pub margin_held: Fixed128,
	pub margin_level: Fixed128,
	pub free_margin: Fixed128,
	pub unrealized_pl: Fixed128,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Eq, PartialEq, Default)]
pub struct PoolInfo<Fixed128> {
	pub enp: Fixed128,
	pub ell: Fixed128,
}

sp_api::decl_runtime_apis! {
	pub trait MarginProtocolApi<AccountId, Fixed128, LiquidityPoolId> where
		AccountId: Codec,
		Fixed128: Codec,
		LiquidityPoolId: Codec,
	{
		fn equity_of_trader(who: AccountId) -> Option<Fixed128>;
		fn margin_level(who: AccountId) -> Option<Fixed128>;
		fn free_margin(who: AccountId) -> Option<Fixed128>;
		fn margin_held(who: AccountId) -> Fixed128;
		fn unrealized_pl_of_trader(who: AccountId) -> Option<Fixed128>;
		fn trader_info(who: AccountId) -> TraderInfo<Fixed128>;
		fn pool_info(pool_id: LiquidityPoolId) -> PoolInfo<Fixed128>;
	}
}
