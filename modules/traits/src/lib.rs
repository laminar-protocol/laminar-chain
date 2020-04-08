#![cfg_attr(not(feature = "std"), no_std)]

use orml_utilities::Fixed128;
use primitives::{Balance, CurrencyId, Leverage, LiquidityPoolId, TradingPair};
use sp_runtime::{DispatchError, DispatchResult, Permill};
use sp_std::{prelude::*, result};

pub trait LiquidityPools<AccountId> {
	fn all() -> Vec<LiquidityPoolId>;
	fn is_owner(pool_id: LiquidityPoolId, who: &AccountId) -> bool;

	/// Return collateral balance of `pool_id`.
	fn liquidity(pool_id: LiquidityPoolId) -> Balance;
	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(source: &AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult;
	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(dest: &AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult;
}

pub trait LiquidityPoolManager<LiquidityPoolId, Balance> {
	fn can_remove(pool_id: LiquidityPoolId) -> bool;
	fn get_required_deposit(pool_id: LiquidityPoolId) -> result::Result<Balance, DispatchError>;
	fn ensure_can_withdraw(pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult;
}

pub trait SyntheticProtocolLiquidityPools<AccountId>: LiquidityPools<AccountId> {
	fn get_bid_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Permill>;
	fn get_ask_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Permill>;
	fn get_additional_collateral_ratio(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Permill;
	fn can_mint(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> bool;
}

pub trait MarginProtocolLiquidityPools<AccountId>: LiquidityPools<AccountId> {
	fn is_allowed_position(pool_id: LiquidityPoolId, pair: TradingPair, leverage: Leverage) -> bool;
	fn get_bid_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Permill>;
	fn get_ask_spread(pool_id: LiquidityPoolId, pair: TradingPair) -> Option<Permill>;
	fn get_swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> Fixed128;
	/// Accumulated swap rate, with USD account currency.
	fn get_accumulated_swap_rate(pool_id: LiquidityPoolId, pair: TradingPair, is_long: bool) -> Fixed128;
	fn can_open_position(
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		leverage: Leverage,
		leveraged_amount: Balance,
	) -> bool;
}

pub trait OnDisableLiquidityPool {
	fn on_disable(pool_id: LiquidityPoolId);
}

pub trait OnRemoveLiquidityPool {
	fn on_remove(pool_id: LiquidityPoolId);
}

pub trait Treasury<AccountId> {
	fn account_id() -> AccountId;
}
