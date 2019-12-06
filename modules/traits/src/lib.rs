#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use frame_support::Parameter;
use rstd::{fmt::Debug, result};
use sp_runtime::{
	traits::{MaybeSerializeDeserialize, Member, SimpleArithmetic},
	Permill,
};

pub trait Leverage {
	fn get_value(&self) -> u8;
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

pub trait LiquidityPoolsCurrency<AccountId>: LiquidityPoolBaseTypes {
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type Error: Into<&'static str> + Debug;

	/// Check collateral balance of `pool_id`.
	fn balance(pool_id: Self::LiquidityPoolId) -> Self::Balance;

	/// Deposit some amount of collateral to `pool_id`, from `who`.
	fn deposit(
		from: &AccountId,
		pool_id: Self::LiquidityPoolId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error>;

	/// Withdraw some amount of collateral to `who`, from `pool_id`.
	fn withdraw(
		to: &AccountId,
		pool_id: Self::LiquidityPoolId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error>;
}

pub trait LiquidityPools<AccountId>:
	LiquidityPoolsConfig + LiquidityPoolsPosition + LiquidityPoolsCurrency<AccountId>
{
}

pub trait LiquidityPoolManager<LiquidityPoolId> {
	fn can_remove(pool: LiquidityPoolId) -> bool;
}
