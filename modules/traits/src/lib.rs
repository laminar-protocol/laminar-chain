#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use rstd::fmt::Debug;
use sr_primitives::{traits::MaybeSerializeDeserialize, Permill};

pub trait Leverage {
	fn get_value() -> u8;
	fn is_long(&self) -> bool;
	fn is_short(&self) -> bool {
		!self.is_long()
	}
}

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
	type Leverage: Leverage;

	fn is_allowed_position(
		pool_id: Self::LiquidityPoolId,
		currency_id: Self::CurrencyId,
		leverage: Self::Leverage,
	) -> bool;
}

pub trait LiquidityPools: LiquidityPoolsConfig + LiquidityPoolsPosition {}
