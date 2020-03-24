#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use frame_support::Parameter;
use orml_utilities::Fixed128;
use primitives::{Balance, Leverage, TradingPair};
use sp_runtime::{
	traits::{AtLeast32Bit, MaybeSerializeDeserialize},
	DispatchError, DispatchResult, Permill,
};
use sp_std::{fmt::Debug, prelude::*, result};

pub trait LiquidityPools<AccountId> {
	type LiquidityPoolId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;
	type CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;
	type Balance: Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	fn ensure_liquidity(pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult;

	fn is_owner(pool_id: Self::LiquidityPoolId, who: &AccountId) -> bool;

	/// Return collateral balance of `pool_id`.
	fn liquidity(pool_id: Self::LiquidityPoolId) -> Self::Balance;
	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(source: &AccountId, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult;
	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(dest: &AccountId, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult;
}

pub trait LiquidityPoolManager<LiquidityPoolId, Balance> {
	fn can_remove(pool_id: LiquidityPoolId) -> bool;
	fn get_required_deposit(pool_id: LiquidityPoolId) -> result::Result<Balance, DispatchError>;
	fn ensure_can_withdraw(pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult;
}

pub trait SyntheticProtocolLiquidityPools<AccountId>: LiquidityPools<AccountId> {
	fn get_bid_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill>;
	fn get_ask_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill>;
	fn get_additional_collateral_ratio(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Permill;
	fn can_mint(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> bool;
}

pub trait MarginProtocolLiquidityPools<AccountId>: LiquidityPools<AccountId> {
	type TradingPair;
	fn is_allowed_position(pool_id: Self::LiquidityPoolId, pair: TradingPair, leverage: Leverage) -> bool;
	fn get_bid_spread(pool_id: Self::LiquidityPoolId, pair: TradingPair) -> Option<Permill>;
	fn get_ask_spread(pool_id: Self::LiquidityPoolId, pair: TradingPair) -> Option<Permill>;
	fn get_swap_rate(pool_id: Self::LiquidityPoolId, pair: Self::TradingPair) -> Fixed128;
	/// Accumulated swap rate, with USD account currency.
	fn get_accumulated_swap_rate(pool_id: Self::LiquidityPoolId, pair: Self::TradingPair) -> Fixed128;
	fn can_open_position(
		pool_id: Self::LiquidityPoolId,
		pair: Self::TradingPair,
		leverage: Leverage,
		leveraged_amount: Balance,
	) -> bool;
}

pub trait Treasry<AccountId> {
	fn account_id() -> AccountId;
}
