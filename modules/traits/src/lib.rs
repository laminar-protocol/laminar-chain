#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use frame_support::Parameter;
use primitives::Leverage;
use sp_runtime::{
	traits::{MaybeSerializeDeserialize, Member, SimpleArithmetic},
	DispatchResult, Permill,
};
use sp_std::fmt::Debug;

pub trait LiquidityPoolBaseTypes {
	type LiquidityPoolId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;
	type CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;
}

pub trait LiquidityPoolsConfig<AccountId>: LiquidityPoolBaseTypes {
	fn get_bid_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill>;
	fn get_ask_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill>;
	fn get_additional_collateral_ratio(
		pool_id: Self::LiquidityPoolId,
		currency_id: Self::CurrencyId,
	) -> Option<Permill>;

	fn is_owner(pool_id: Self::LiquidityPoolId, who: &AccountId) -> bool;
}

pub trait LiquidityPoolsPosition: LiquidityPoolBaseTypes {
	fn is_allowed_position(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId, leverage: Leverage) -> bool;
}

pub trait LiquidityPoolsCurrency<AccountId>: LiquidityPoolBaseTypes {
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;

	/// Check collateral balance of `pool_id`.
	fn balance(pool_id: Self::LiquidityPoolId) -> Self::Balance;

	/// Deposit some amount of collateral to `pool_id`, from `who`.
	fn deposit(from: &AccountId, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult;

	/// Withdraw some amount of collateral to `who`, from `pool_id`.
	fn withdraw(to: &AccountId, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult;
}

pub trait LiquidityPools<AccountId>:
	LiquidityPoolsConfig<AccountId> + LiquidityPoolsPosition + LiquidityPoolsCurrency<AccountId>
{
}

pub trait LiquidityPoolManager<LiquidityPoolId> {
	fn can_remove(pool: LiquidityPoolId) -> bool;
}
