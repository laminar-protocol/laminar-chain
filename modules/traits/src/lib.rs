#![cfg_attr(not(feature = "std"), no_std)]

use primitives::{Balance, CurrencyId, Leverage, LiquidityPoolId, TradingPair};
use sp_arithmetic::FixedI128;
use sp_runtime::{DispatchResult, Permill};
use sp_std::prelude::*;

/// An abstraction of liquidity pools basic functionalities.
pub trait LiquidityPools<AccountId> {
	/// Return all liquidity pools.
	fn all() -> Vec<LiquidityPoolId>;

	/// Return `true` if `who` is owner of `pool_id`.
	fn is_owner(pool_id: LiquidityPoolId, who: &AccountId) -> bool;

	/// Return `true` if `pool_id` exists.
	fn pool_exists(pool_id: LiquidityPoolId) -> bool;

	/// Return liquidity balance of `pool_id`.
	fn liquidity(pool_id: LiquidityPoolId) -> Balance;

	/// Deposit liquidity from `source` to `pool_id` of the given amount.
	fn deposit_liquidity(source: &AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult;

	/// Withdraw liquidity from `pool_id` to `dest` of the given amount.
	fn withdraw_liquidity(dest: &AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult;
}

/// An abstraction of base liquidity pools manager.
pub trait BaseLiquidityPoolManager<LiquidityPoolId, Balance> {
	/// Check if pool can be removed.
	fn can_remove(pool_id: LiquidityPoolId) -> bool;

	/// Return `Ok` iff the account is able to make a withdrawal of the given amount.
	/// Basically, it's just a dry-run of `withdraw`.
	fn ensure_can_withdraw(pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult;
}

/// An abstraction of liquidity pools for Synthetic Protocol.
pub trait SyntheticProtocolLiquidityPools<AccountId>: LiquidityPools<AccountId> {
	/// Return bid spread of `currency_id` in `pool_id`, or `None` if not set by pool owner.
	fn bid_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Balance>;

	/// Return ask spread of `currency_id` in `pool_id`, or `None` if not set by pool owner.
	fn ask_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Balance>;

	/// Return additional collateral ratio of `currency_id`.
	fn additional_collateral_ratio(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Permill;

	/// Return `true` if `currency_id` can be minted in `pool_id`.
	fn can_mint(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> bool;
}

/// An abstraction of liquidity pools for Margin Protocol.
pub trait MarginProtocolLiquidityPools<AccountId>: LiquidityPools<AccountId> {
	/// Returns `true` if `leverage` of `pair` is allowed in `pool_id`.
	fn is_allowed_leverage(pool_id: LiquidityPoolId, pair: TradingPair, leverage: Leverage) -> bool;

	/// Returns bid spread of `pair` in `pool_id`, or `None` if not set by pool owner.
	fn bid_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Balance>;

	/// Returns ask spread of `pair` in `pool_id`, or `None` if not set by pool owner.
	fn ask_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Balance>;

	/// Returns swap rate of `pair` in `pool_id`.
	fn swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> FixedI128;

	/// Return accumulated swap rate by USD.
	fn accumulated_swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> FixedI128;

	/// Return `true` if position can be opened in `pool_id`.
	fn can_open_position(
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		leverage: Leverage,
		leveraged_amount: Balance,
	) -> bool;
}

/// Margin protocol liquidity pools manager.
pub trait MarginProtocolLiquidityPoolsManager {
	/// Return `Ok` iff the trading pair could be enabled in `pool_id`.
	fn ensure_can_enable_trading_pair(pool_id: LiquidityPoolId, pair: TradingPair) -> DispatchResult;
}

/// The liquidity pool was disabled by owner.
pub trait OnDisableLiquidityPool {
	/// Invoked when the liquiditiy pool has been disabled.
	fn on_disable(pool_id: LiquidityPoolId);
}

/// The liquidity pool was removed by owner.
pub trait OnRemoveLiquidityPool {
	/// Invoked when the liquiditiy pool has been removed.
	fn on_remove(pool_id: LiquidityPoolId);
}

/// The laminar treasury abstraction.
pub trait Treasury<AccountId> {
	/// Account ID of treasury.
	fn account_id() -> AccountId;
}
