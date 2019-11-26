#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use num_traits::Signed;
use rstd::fmt::Debug;
use sr_primitives::{traits::MaybeSerializeDeserialize, Permill};

pub trait LiquidityPoolBaseTypes {
	type LiquidityPoolId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;
	type CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;
}

pub trait LiquidityPoolsConfig: LiquidityPoolBaseTypes {
	fn get_bid_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Permill;
	fn get_ask_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Permill;
	fn get_additional_collateral_ratio(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Permill;
}

pub trait LiquidityPoolsPosition: LiquidityPoolBaseTypes {
	/// Signed leverage type: positive means long and negative means short.
	type Leverage: Signed + FullCodec + Eq + PartialEq + PartialOrd + Ord + Copy + MaybeSerializeDeserialize + Debug;

	fn is_allowed_position(
		pool_id: Self::LiquidityPoolId,
		currency_id: Self::CurrencyId,
		leverage: Self::Leverage,
	) -> bool;
}

pub trait LiquidityPools: LiquidityPoolsConfig + LiquidityPoolsPosition {}
