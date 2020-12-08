#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	debug, decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{EnsureOrigin, Get},
	weights::{DispatchClass, Weight},
	IterableStorageDoubleMap, IterableStorageMap,
};
use frame_system::{
	ensure_none, ensure_signed,
	offchain::{SendTransactionTypes, SubmitTransaction},
};
use orml_traits::{BasicCurrency, PriceProvider};
use orml_utilities::with_transaction_result;
use primitives::{
	arithmetic::{fixed_i128_from_fixed_u128, fixed_i128_from_u128, fixed_i128_mul_signum, u128_from_fixed_i128},
	Balance, CurrencyId, Leverage, LiquidityPoolId, Price, TradingPair,
};
use sp_arithmetic::{
	traits::{Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Saturating},
	FixedI128, FixedPointNumber, Permill,
};
use sp_runtime::{
	offchain::{
		storage_lock::{StorageLock, Time},
		Duration,
	},
	traits::{AccountIdConversion, StaticLookup},
	transaction_validity::{
		InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity, TransactionValidityError,
		ValidTransaction,
	},
	DispatchError, DispatchResult, ModuleId, RuntimeDebug,
};
use sp_std::{cmp, prelude::*, result};
use traits::{
	BaseLiquidityPoolManager, LiquidityPools, MarginProtocolLiquidityPools, MarginProtocolLiquidityPoolsManager,
	OpenPositionError,
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

mod default_weight;
mod mock;
mod tests;

pub trait WeightInfo {
	fn deposit() -> Weight;
	fn withdraw() -> Weight;
	fn open_position() -> Weight;
	fn open_position_with_ten_in_pool() -> Weight;
	fn close_position() -> Weight;
	fn close_position_with_ten_in_pool() -> Weight;
	fn trader_margin_call() -> Weight;
	fn trader_become_safe() -> Weight;
	fn trader_stop_out() -> Weight;
	fn liquidity_pool_margin_call() -> Weight;
	fn liquidity_pool_become_safe() -> Weight;
	fn liquidity_pool_force_close() -> Weight;
	fn set_trading_pair_risk_threshold() -> Weight;
}

const MODULE_ID: ModuleId = ModuleId(*b"lami/mgn");

pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// The currency used for liquidity.
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Balance>;

	/// The `MarginProtocolLiquidityPools` implementation.
	type LiquidityPools: MarginProtocolLiquidityPools<Self::AccountId>;

	/// Provides market prices.
	type PriceProvider: PriceProvider<CurrencyId, Price>;

	/// The account ID of treasury.
	type GetTreasuryAccountId: Get<Self::AccountId>;

	/// Maximum number of positions one trader could open.
	type GetTraderMaxOpenPositions: Get<usize>;

	/// Maximum number of positions could be opened in a pool.
	type GetPoolMaxOpenPositions: Get<usize>;

	/// Required origin for updating protocol options.
	type UpdateOrigin: EnsureOrigin<Self::Origin>;

	/// A configuration for base priority of unsigned transactions.
	///
	/// This is exposed so that it can be tuned for particular runtime, when
	/// multiple pallets send unsigned transactions.
	type UnsignedPriority: Get<TransactionPriority>;

	/// Weight information for the extrinsics in this module.
	type WeightInfo: WeightInfo;
}

pub type PositionId = u64;

/// Margin protocol Position.
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
pub struct Position<T: Config> {
	/// Owner.
	owner: T::AccountId,

	/// Liquidity pool ID where the position is opened in.
	pool: LiquidityPoolId,

	/// Trader pair.
	pair: TradingPair,

	/// Leverage.
	leverage: Leverage,

	/// Leveraged held amount.
	///
	/// Positive value if long position, negative if short.
	leveraged_held: FixedI128,

	/// Leveraged debits amount.
	///
	/// Negative value if long position, positive if short.
	leveraged_debits: FixedI128,

	/// Accumulated swap rate on open position.
	open_accumulated_swap_rate: FixedI128,

	/// Margin held.
	margin_held: FixedI128,
}

/// Positions snapshot.
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct PositionsSnapshot {
	/// Positions count.
	positions_count: PositionId,

	/// Total long leveraged amounts.
	long: LeveragedAmounts,

	/// Total short leveraged amounts.
	short: LeveragedAmounts,
}

/// Total leveraged amounts in a positions snapshot.
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LeveragedAmounts {
	/// Leveraged held amount.
	held: FixedI128,

	/// Leveraged debits amount.
	debits: FixedI128,
}

/// Risk threshold.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct RiskThreshold {
	/// Margin call threshold.
	pub margin_call: Permill,

	/// Stop out threshold.
	pub stop_out: Permill,
}

/// Risk threshold for a trading pair.
#[derive(Encode, Decode, Copy, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct TradingPairRiskThreshold {
	/// Trader risk threshold.
	pub trader: Option<RiskThreshold>,

	/// Liquidity pool Equity to Net Position Ratio (ENP) threshold.
	pub enp: Option<RiskThreshold>,

	/// Liquidity pool Equity to Longest Leg Ratio (ELL) threshold.
	pub ell: Option<RiskThreshold>,
}
impl TradingPairRiskThreshold {
	pub fn new(trader: Option<RiskThreshold>, enp: Option<RiskThreshold>, ell: Option<RiskThreshold>) -> Self {
		Self { trader, enp, ell }
	}
}

decl_storage! {
	trait Store for Module<T: Config> as MarginProtocol {
		/// Next available position ID.
		NextPositionId get(fn next_position_id): PositionId;

		/// Positions.
		Positions get(fn positions): map hasher(twox_64_concat) PositionId => Option<Position<T>>;

		/// Positions existence check by traders and liquidity pool IDs.
		PositionsByTrader get(fn positions_by_trader): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) (LiquidityPoolId, PositionId) => Option<()>;

		/// Positions existence check by pools and trading pairs.
		PositionsByPool get(fn positions_by_pool): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) (TradingPair, PositionId) => Option<()>;

		/// Positions snapshots.
		///
		/// Used for performance improvement.
		PositionsSnapshots get(fn pool_positions_snapshots): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) TradingPair => PositionsSnapshot;

		/// Balance of a trader in a liquidity pool.
		///
		/// The balance value could be positive or negative:
		/// - If positive, it represents 'balance' the trader could use to open positions, withdraw etc.
		/// - If negative, it represents how much the trader owes the pool. Owing could happen when realizing loss.
		/// but trader has not enough free margin at the moment; Then repayment would be done while realizing profit.
		Balances get(fn balances): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) LiquidityPoolId => FixedI128;

		/// Margin call check of a trader in a pool.
		///
		/// A trader may only open new positions if not in margin called state.
		MarginCalledTraders get(fn margin_called_traders): double_map hasher(twox_64_concat) T::AccountId, hasher(twox_64_concat) LiquidityPoolId => Option<()>;


		/// Margin call pool.
		///
		/// New positions may only be opened in a pool if which not in margin called state.
		MarginCalledPools get(fn margin_called_pools): map hasher(twox_64_concat) LiquidityPoolId => Option<()>;

		/// Risk thresholds of a trading pair, including trader risk threshold, pool ENP and ELL risk threshold.
		///
		/// DEFAULT-NOTE: `trader`, `enp`, and `ell` are all `None` by default.
		RiskThresholds get(fn risk_thresholds): map hasher(twox_64_concat) TradingPair => TradingPairRiskThreshold;
	}

	add_extra_genesis {
		config(risk_thresholds): Vec<(TradingPair, RiskThreshold, RiskThreshold, RiskThreshold)>;
		build(|config: &GenesisConfig| {
			config.risk_thresholds.iter().for_each(|(pair, trader, enp, ell)| {
				RiskThresholds::insert(
					pair,
					TradingPairRiskThreshold::new(Some(*trader), Some(*enp), Some(*ell)),
				);
			})
		})
	}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Config>::AccountId,
		LiquidityPoolId = LiquidityPoolId,
		TradingPair = TradingPair,
		Amount = Balance
	{
		/// Position opened: \[who, position_id, pool_id, pair, leverage, leveraged_amount, open_price\]
		PositionOpened(AccountId, PositionId, LiquidityPoolId, TradingPair, Leverage, Amount, Price),

		/// Position closed: \[who, position_id, pool_id, close_price\]
		PositionClosed(AccountId, PositionId, LiquidityPoolId, Price),

		/// Deposited: \[who, pool_id, amount\]
		Deposited(AccountId, LiquidityPoolId, Amount),

		/// Withdrew: \[who, pool_id, amount\]
		Withdrew(AccountId, LiquidityPoolId, Amount),

		/// Trader margin called: \[who\]
		TraderMarginCalled(AccountId),

		/// Trader became safe: \[who\]
		TraderBecameSafe(AccountId),

		/// Trader stopped out: \[who\]
		TraderStoppedOut(AccountId),

		/// Liquidity pool margin called: \[pool_id\]
		LiquidityPoolMarginCalled(LiquidityPoolId),

		/// Liquidity pool became safe: \[pool_id\]
		LiquidityPoolBecameSafe(LiquidityPoolId),

		/// Liquidity pool force closed: \[pool_id\]
		LiquidityPoolForceClosed(LiquidityPoolId),

		/// Trading pair risk threshold set: \[pair, trader_risk_threshold, liquidity_pool_enp_threshold, liquidity_pool_ell_threshold\]
		TradingPairRiskThresholdSet(TradingPair, Option<RiskThreshold>, Option<RiskThreshold>, Option<RiskThreshold>),
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// No price from provider.
		NoPrice,

		/// Ask spread not set.
		NoAskSpread,

		/// Bid spread not set.
		NoBidSpread,

		/// Market price is too high.
		MarketPriceTooHigh,

		/// Market price is too low.
		MarketPriceTooLow,

		/// Number out of bound in calculation.
		NumOutOfBound,

		/// Trader is not safe.
		UnsafeTrader,

		/// Liquidity pool is not safe.
		UnsafePool,

		/// Pool would be unsafe.
		PoolWouldBeUnsafe,

		/// Trader is safe.
		SafeTrader,

		/// Pool is safe.
		SafePool,

		/// Not reach risk threshold yet.
		NotReachedRiskThreshold,

		/// Trader has been margin called.
		MarginCalledTrader,

		/// Pool has been margin called.
		MarginCalledPool,

		/// No available position id.
		NoAvailablePositionId,

		/// Position not found.
		PositionNotFound,

		/// Position is not opened by caller.
		PositionNotOpenedByTrader,

		/// Leverage not allowed in pool,
		LeverageNotAllowedInPool,

		/// Trading pair not enabled in protocol,
		TradingPairNotEnabled,

		/// Trading pair not enabled in pool,
		TradingPairNotEnabledInPool,

		/// Leveraged amount is below mininum,
		BelowMinLeveragedAmount,

		/// Positions count reached maximum.
		CannotOpenMorePosition,

		/// Insufficient free margin.
		InsufficientFreeMargin,

		/// Risk threshold not set.
		NoRiskThreshold,
	}
}

impl<T: Config> From<OpenPositionError> for Error<T> {
	fn from(error: OpenPositionError) -> Self {
		match error {
			OpenPositionError::LeverageNotAllowedInPool => Error::<T>::LeverageNotAllowedInPool,
			OpenPositionError::TradingPairNotEnabled => Error::<T>::TradingPairNotEnabled,
			OpenPositionError::TradingPairNotEnabledInPool => Error::<T>::TradingPairNotEnabledInPool,
			OpenPositionError::BelowMinLeveragedAmount => Error::<T>::BelowMinLeveragedAmount,
		}
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const GetTreasuryAccountId: T::AccountId = T::GetTreasuryAccountId::get();
		const GetTraderMaxOpenPositions: u32 = T::GetTraderMaxOpenPositions::get() as u32;
		const GetPoolMaxOpenPositions: u32 = T::GetPoolMaxOpenPositions::get() as u32;
		const UnsignedPriority: TransactionPriority = T::UnsignedPriority::get();

		/// Open a position in `pool_id`.
		#[weight = T::WeightInfo::open_position()]
		pub fn open_position(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			pair: TradingPair,
			leverage: Leverage,
			#[compact] leveraged_amount: Balance,
			price: Price,
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_open_position(&who, pool_id, pair, leverage, leveraged_amount, price)?;
				Ok(())
			})?;
		}

		/// Close position by id.
		#[weight = T::WeightInfo::close_position()]
		pub fn close_position(origin, #[compact] position_id: PositionId, price: Price) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_close_position(&who, position_id, Some(price))?;
				Ok(())
			})?;
		}

		/// Deposit liquidity to caller's account.
		#[weight = T::WeightInfo::deposit()]
		pub fn deposit(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_deposit(&who, pool_id, amount)?;
				Self::deposit_event(RawEvent::Deposited(who, pool_id, amount));
				Ok(())
			})?;
		}

		/// Withdraw liquidity from caller's account.
		#[weight = T::WeightInfo::withdraw()]
		pub fn withdraw(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_withdraw(&who, pool_id, amount)?;
				Self::deposit_event(RawEvent::Withdrew(who, pool_id, amount));
				Ok(())
			})?;
		}

		/// Margin call a trader.
		///
		/// May only be called from none origin. Would fail if the trader is still safe.
		#[weight = (T::WeightInfo::trader_margin_call(), DispatchClass::Operational)]
		pub fn trader_margin_call(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			#[compact] pool_id: LiquidityPoolId
		) {
			with_transaction_result(|| {
				ensure_none(origin)?;
				let who = T::Lookup::lookup(who)?;

				Self::do_trader_margin_call(&who, pool_id)?;
				Self::deposit_event(RawEvent::TraderMarginCalled(who));

				Ok(())
			})?;
		}

		/// Remove trader's margin-called status.
		///
		/// May only be called from none origin. Would fail if the trader is not safe yet.
		#[weight = T::WeightInfo::trader_become_safe()]
		pub fn trader_become_safe(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			#[compact] pool_id: LiquidityPoolId
		) {
			with_transaction_result(|| {
				ensure_none(origin)?;
				let who = T::Lookup::lookup(who)?;

				Self::do_trader_become_safe(&who, pool_id)?;
				Self::deposit_event(RawEvent::TraderBecameSafe(who));

				Ok(())
			})?;
		}

		/// Stop out a trader.
		///
		/// May only be called from none origin. Would fail if stop out threshold not reached.
		#[weight = (T::WeightInfo::trader_stop_out(), DispatchClass::Operational)]
		pub fn trader_stop_out(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			#[compact] pool_id: LiquidityPoolId
		) {
			with_transaction_result(|| {
				ensure_none(origin)?;
				let who = T::Lookup::lookup(who)?;

				Self::do_trader_stop_out(&who, pool_id)?;
				Self::deposit_event(RawEvent::TraderStoppedOut(who));

				Ok(())
			})?;
		}

		/// Margin call a liquidity pool.
		///
		/// May only be called from none origin. Would fail if the pool still safe.
		#[weight = (T::WeightInfo::liquidity_pool_margin_call(), DispatchClass::Operational)]
		pub fn liquidity_pool_margin_call(origin, #[compact] pool: LiquidityPoolId) {
			with_transaction_result(|| {
				ensure_none(origin)?;
				Self::do_liquidity_pool_margin_call(pool)?;
				Self::deposit_event(RawEvent::LiquidityPoolMarginCalled(pool));
				Ok(())
			})?;
		}

		/// Remove a pool's margin-called status.
		///
		/// May only be called from none origin. Would fail if the pool is not safe yet.
		#[weight = T::WeightInfo::liquidity_pool_become_safe()]
		pub fn liquidity_pool_become_safe(origin, #[compact] pool: LiquidityPoolId) {
			with_transaction_result(|| {
				ensure_none(origin)?;
				Self::do_liquidity_pool_become_safe(pool)?;
				Self::deposit_event(RawEvent::LiquidityPoolBecameSafe(pool));
				Ok(())
			})?;
		}

		/// Force close a liquidity pool.
		///
		/// May only be called from none origin. Would fail if pool ENP or ELL thresholds not reached.
		#[weight = (T::WeightInfo::liquidity_pool_force_close(), DispatchClass::Operational)]
		pub fn liquidity_pool_force_close(origin, #[compact] pool: LiquidityPoolId) {
			with_transaction_result(|| {
				ensure_none(origin)?;
				Self::do_liquidity_pool_force_close(pool)?;
				Self::deposit_event(RawEvent::LiquidityPoolForceClosed(pool));
				Ok(())
			})?;
		}

		/// Set risk thresholds of a trading pair.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_trading_pair_risk_threshold()]
		pub fn set_trading_pair_risk_threshold(
			origin,
			pair: TradingPair,
			trader: Option<RiskThreshold>,
			enp: Option<RiskThreshold>,
			ell: Option<RiskThreshold>
		) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;

				RiskThresholds::mutate(pair, |r| {
					if trader.is_some() {
						r.trader = trader;
					}
					if enp.is_some() {
						r.enp = enp;
					}
					if ell.is_some() {
						r.ell = ell;
					}
				});

				Self::deposit_event(RawEvent::TradingPairRiskThresholdSet(pair, trader, enp, ell));

				Ok(())
			})?;
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			if let Err(error) = Self::offchain_worker(block_number) {
				match error {
					OffchainErr::NotValidator | OffchainErr::OffchainLock => {
						debug::native::info!(
							target: TAG,
							"{:?} [block_number = {:?}]",
							error,
							block_number,
						);
					},
					_ => {
						debug::native::error!(
							target: TAG,
							"{:?} [block_number = {:?}]",
							error,
							block_number,
						);
					}
				};
			}
		}
	}
}

// Storage getters
impl<T: Config> Module<T> {
	pub fn trader_risk_threshold(pair: TradingPair) -> Option<RiskThreshold> {
		Self::risk_thresholds(pair).trader
	}

	pub fn liquidity_pool_enp_threshold(pair: TradingPair) -> Option<RiskThreshold> {
		Self::risk_thresholds(pair).enp
	}

	pub fn liquidity_pool_ell_threshold(pair: TradingPair) -> Option<RiskThreshold> {
		Self::risk_thresholds(pair).ell
	}
}

// Dispatchable calls implementation
impl<T: Config> Module<T> {
	fn do_open_position(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		leverage: Leverage,
		leveraged_amount: Balance,
		price: Price,
	) -> DispatchResult {
		Self::ensure_can_open_more_position(who, pool_id, pair)?;
		ensure!(
			Self::margin_called_traders(who, pool_id).is_none(),
			Error::<T>::MarginCalledTrader
		);
		ensure!(
			Self::margin_called_pools(pool_id).is_none(),
			Error::<T>::MarginCalledPool
		);

		let (held_signum, debit_signum): (i128, i128) = if leverage.is_long() { (1, -1) } else { (-1, 1) };
		let leveraged_held = fixed_i128_from_u128(leveraged_amount);
		let debits_price = {
			if leverage.is_long() {
				Self::ask_price(pool_id, pair, Some(price))?
			} else {
				Self::bid_price(pool_id, pair, Some(price))?
			}
		};
		let leveraged_debits = leveraged_held
			.checked_mul(&debits_price)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let leveraged_held_in_usd = Self::usd_value(pair.quote, leveraged_debits)?;
		T::LiquidityPools::ensure_can_open_position(
			pool_id,
			pair,
			leverage,
			u128_from_fixed_i128(leveraged_held_in_usd),
		)
		.map_err::<Error<T>, _>(|e| e.into())?;

		let margin_held = {
			let leverage_value = FixedI128::saturating_from_integer(leverage.value());
			leveraged_held_in_usd
				.checked_div(&leverage_value)
				.expect("leveraged value cannot be zero; qed")
		};
		let open_accumulated_swap_rate = T::LiquidityPools::accumulated_swap_rate(pool_id, pair, leverage.is_long());
		let position: Position<T> = Position {
			owner: who.clone(),
			pool: pool_id,
			pair,
			leverage,
			leveraged_held: fixed_i128_mul_signum(leveraged_held, held_signum),
			leveraged_debits: fixed_i128_mul_signum(leveraged_debits, debit_signum),
			open_accumulated_swap_rate,
			margin_held,
		};

		let free_margin = Self::free_margin(who, pool_id)?;
		ensure!(free_margin >= margin_held, Error::<T>::InsufficientFreeMargin);
		Self::ensure_trader_safe(who, pool_id, Action::OpenPosition(position.clone()))?;
		Self::ensure_pool_safe(pool_id, Action::OpenPosition(position.clone()))?;

		let id = Self::insert_position(who, pool_id, pair, position)?;

		Self::deposit_event(RawEvent::PositionOpened(
			who.clone(),
			id,
			pool_id,
			pair,
			leverage,
			leveraged_amount,
			Price::from_inner(u128_from_fixed_i128(debits_price)),
		));

		Ok(())
	}

	fn do_close_position(who: &T::AccountId, position_id: PositionId, price: Option<Price>) -> DispatchResult {
		let position = Self::positions(position_id).ok_or(Error::<T>::PositionNotFound)?;
		ensure!(
			<PositionsByTrader<T>>::contains_key(who, (position.pool, position_id)),
			Error::<T>::PositionNotOpenedByTrader
		);
		let (unrealized_pl, market_price) = Self::unrealized_pl_and_market_price_of_position(&position, price)?;
		let accumulated_swap_rate = Self::accumulated_swap_rate_of_position(&position)?;
		let unrealized = unrealized_pl
			.checked_add(&accumulated_swap_rate)
			.ok_or(Error::<T>::NumOutOfBound)?;

		if unrealized.is_positive() {
			// Realize trader's profit.

			let pool_liquidity = fixed_i128_from_u128(<T::LiquidityPools as LiquidityPools<T::AccountId>>::liquidity(
				position.pool,
			));
			// Max realizable is the pool's liquidity.
			let realizable = cmp::min(pool_liquidity, unrealized);

			let mut pool_withdraw = realizable;
			// If negative balance, the trader owes pool and then repay (the amount of negative balance).
			// Note less withdraw(owing < realizable) or no withdraw(owing >= realizable) is the way of
			// repayment.
			let balance = Self::balances(who, position.pool);
			if balance.is_negative() {
				pool_withdraw = cmp::max(pool_withdraw.saturating_add(balance), FixedI128::zero());
			}
			if !pool_withdraw.is_zero() {
				<T::LiquidityPools as LiquidityPools<T::AccountId>>::withdraw_liquidity(
					&Self::account_id(),
					position.pool,
					u128_from_fixed_i128(pool_withdraw),
				)?;
			}

			Self::update_balance(who, position.pool, realizable);
		} else {
			// Realize trader's loss.

			let equity = Self::equity_of_trader(who, position.pool)?;
			let unrealized_abs = unrealized.saturating_abs();
			// Max realizable is the trader's equity excluding this lossy position.
			let realizable = cmp::min(
				cmp::max(equity.saturating_add(unrealized_abs), FixedI128::zero()),
				unrealized_abs,
			);

			// If trader has not enough balance to pay the loss, pool won't get full payment for now. Repayment
			// will happen on close profitable positions later.
			let pool_deposit = cmp::min(
				cmp::max(Self::balances(who, position.pool), FixedI128::zero()),
				realizable,
			);
			if !pool_deposit.is_zero() {
				<T::LiquidityPools as LiquidityPools<T::AccountId>>::deposit_liquidity(
					&Self::account_id(),
					position.pool,
					u128_from_fixed_i128(pool_deposit),
				)?;
			}

			Self::update_balance(who, position.pool, fixed_i128_mul_signum(realizable, -1));
		}

		// Remove position storage operation.
		Self::remove_position(who, position_id, &position)?;

		Self::deposit_event(RawEvent::PositionClosed(
			who.clone(),
			position_id,
			position.pool,
			Price::from_inner(u128_from_fixed_i128(market_price)),
		));

		Ok(())
	}

	fn do_deposit(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount)?;
		Self::update_balance(who, pool_id, fixed_i128_from_u128(amount));

		Ok(())
	}

	fn do_withdraw(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		let free_margin = Self::free_margin(who, pool_id)?;
		let amount_fixedi128 = fixed_i128_from_u128(amount);
		ensure!(free_margin >= amount_fixedi128, Error::<T>::InsufficientFreeMargin);

		T::LiquidityCurrency::transfer(&Self::account_id(), who, amount)?;
		Self::update_balance(who, pool_id, fixed_i128_mul_signum(amount_fixedi128, -1));

		Ok(())
	}

	fn do_trader_margin_call(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		if !Self::is_trader_margin_called(who, pool_id) {
			if Self::ensure_trader_safe(who, pool_id, Action::None).is_err() {
				<MarginCalledTraders<T>>::insert(who, pool_id, ());
			} else {
				return Err(Error::<T>::SafeTrader.into());
			}
		}
		Ok(())
	}

	fn do_trader_become_safe(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		if Self::is_trader_margin_called(who, pool_id) {
			if Self::ensure_trader_safe(who, pool_id, Action::None).is_ok() {
				<MarginCalledTraders<T>>::remove(who, pool_id);
			} else {
				return Err(Error::<T>::UnsafeTrader.into());
			}
		}
		Ok(())
	}

	fn do_trader_stop_out(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		let risk = Self::check_trader(who, pool_id, Action::None)?;
		match risk {
			Risk::StopOut => {
				// To stop out a trader:
				//   1. Close the position with the biggest loss.
				//   2. Repeat step 1 until no stop out risk, or all positions of this trader has been closed.

				let mut positions: Vec<(PositionId, FixedI128)> = <PositionsByTrader<T>>::iter_prefix(who)
					.filter_map(|((_, position_id), _)| {
						let position = Self::positions(position_id)?;
						if position.pool != pool_id {
							return None;
						}

						let unrealized_pl = Self::unrealized_pl_of_position(&position).ok()?;
						let accumulated_swap_rate = Self::accumulated_swap_rate_of_position(&position).ok()?;
						let unrealized = unrealized_pl.checked_add(&accumulated_swap_rate)?;
						Some((position_id, unrealized))
					})
					.collect();
				positions.sort_by(|x, y| x.1.cmp(&y.1));

				for (id, _) in positions {
					let _ = Self::do_close_position(who, id, None);
					let new_risk = Self::check_trader(who, pool_id, Action::None)?;
					match new_risk {
						Risk::StopOut => {}
						_ => break,
					}
				}

				if Self::ensure_trader_safe(who, pool_id, Action::None).is_ok()
					&& Self::is_trader_margin_called(who, pool_id)
				{
					<MarginCalledTraders<T>>::remove(who, pool_id);
				}
				Ok(())
			}
			_ => Err(Error::<T>::NotReachedRiskThreshold.into()),
		}
	}

	fn do_liquidity_pool_margin_call(pool: LiquidityPoolId) -> DispatchResult {
		if !Self::is_pool_margin_called(&pool) {
			if Self::ensure_pool_safe(pool, Action::None).is_err() {
				MarginCalledPools::insert(pool, ());
			} else {
				return Err(Error::<T>::SafePool.into());
			}
		}
		Ok(())
	}

	fn do_liquidity_pool_become_safe(pool: LiquidityPoolId) -> DispatchResult {
		if Self::is_pool_margin_called(&pool) {
			if Self::ensure_pool_safe(pool, Action::None).is_ok() {
				MarginCalledPools::remove(pool);
			} else {
				return Err(Error::<T>::UnsafePool.into());
			}
		}
		Ok(())
	}

	fn do_liquidity_pool_force_close(pool: LiquidityPoolId) -> DispatchResult {
		match Self::check_pool(pool, Action::None) {
			Ok(Risk::StopOut) => {
				PositionsByPool::iter_prefix(pool).for_each(|((_, position_id), _)| {
					let _ = Self::liquidity_pool_close_position(pool, position_id);
				});

				if Self::ensure_pool_safe(pool, Action::None).is_ok() && Self::is_pool_margin_called(&pool) {
					MarginCalledPools::remove(pool);
				}
				Ok(())
			}
			_ => Err(Error::<T>::NotReachedRiskThreshold.into()),
		}
	}
}

// Storage helpers
impl<T: Config> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	fn insert_position(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		pair: TradingPair,
		position: Position<T>,
	) -> result::Result<PositionId, DispatchError> {
		let id = Self::next_position_id();
		ensure!(id != PositionId::max_value(), Error::<T>::NoAvailablePositionId);

		PositionsSnapshots::try_mutate(pool_id, pair, |snapshot| -> DispatchResult {
			if position.leverage.is_long() {
				snapshot.positions_count += 1;
				snapshot.long.held = snapshot
					.long
					.held
					.checked_add(&position.leveraged_held)
					.ok_or(Error::<T>::NumOutOfBound)?;
				snapshot.long.debits = snapshot
					.long
					.debits
					.checked_add(&position.leveraged_debits)
					.ok_or(Error::<T>::NumOutOfBound)?;
			} else {
				snapshot.positions_count += 1;
				snapshot.short.held = snapshot
					.long
					.held
					.checked_add(&position.leveraged_held)
					.ok_or(Error::<T>::NumOutOfBound)?;
				snapshot.short.debits = snapshot
					.long
					.debits
					.checked_add(&position.leveraged_debits)
					.ok_or(Error::<T>::NumOutOfBound)?;
			}
			Ok(())
		})?;

		NextPositionId::mutate(|id| *id += 1);

		<Positions<T>>::insert(id, position);
		<PositionsByTrader<T>>::insert(who, (pool_id, id), ());
		PositionsByPool::insert(pool_id, (pair, id), ());

		Ok(id)
	}

	fn remove_position(
		who: &T::AccountId,
		position_id: PositionId,
		position: &Position<T>,
	) -> result::Result<(), DispatchError> {
		<Positions<T>>::remove(position_id);
		<PositionsByTrader<T>>::remove(who, (position.pool, position_id));
		PositionsByPool::remove(position.pool, (position.pair, position_id));

		PositionsSnapshots::mutate(position.pool, position.pair, |snapshot| {
			if position.leverage.is_long() {
				snapshot.positions_count -= 1;
				snapshot.long.held = snapshot
					.long
					.held
					.checked_sub(&position.leveraged_held)
					.expect("pool amount can't overflow; qed");
				snapshot.long.debits = snapshot
					.long
					.debits
					.checked_sub(&position.leveraged_debits)
					.expect("pool amount can't overflow; qed");
			} else {
				snapshot.positions_count -= 1;
				snapshot.short.held = snapshot
					.short
					.held
					.checked_sub(&position.leveraged_held)
					.expect("pool amount can't overflow; qed");
				snapshot.short.debits = snapshot
					.short
					.debits
					.checked_sub(&position.leveraged_debits)
					.expect("pool amount can't overflow; qed");
			}
		});

		// reset trader's equity to $0
		let has_position = <PositionsByTrader<T>>::iter_prefix(who).any(|((p, _), _)| p == position.pool);

		if !has_position && Self::balances(who, position.pool).is_negative() {
			<Balances<T>>::remove(who, position.pool);
		}

		Ok(())
	}

	/// Update `who` balance in `pool_id` by `amount`.
	///
	/// Note this function guarantees op, don't use in possible no-op scenario.
	fn update_balance(who: &T::AccountId, pool_id: LiquidityPoolId, amount: FixedI128) {
		let new_balance = Self::balances(who, pool_id).saturating_add(amount);
		<Balances<T>>::insert(who, pool_id, new_balance);
	}

	fn ensure_can_open_more_position(who: &T::AccountId, pool: LiquidityPoolId, pair: TradingPair) -> DispatchResult {
		ensure!(
			(Self::pool_positions_snapshots(pool, pair).positions_count as usize) < T::GetPoolMaxOpenPositions::get(),
			Error::<T>::CannotOpenMorePosition
		);
		let count = <PositionsByTrader<T>>::iter_prefix(who)
			.filter(|((p, _), _)| *p == pool)
			.count();
		ensure!(
			count < T::GetTraderMaxOpenPositions::get(),
			Error::<T>::CannotOpenMorePosition
		);
		Ok(())
	}
}

type PriceResult = result::Result<Price, DispatchError>;
type FixedI128Result = result::Result<FixedI128, DispatchError>;
type DoubleFixedI128Result = result::Result<(FixedI128, FixedI128), DispatchError>;

// Price helpers
impl<T: Config> Module<T> {
	/// The price from oracle.
	fn price(base: CurrencyId, quote: CurrencyId) -> PriceResult {
		T::PriceProvider::get_price(base, quote).ok_or_else(|| Error::<T>::NoPrice.into())
	}

	/// ask_price = price + ask_spread
	fn ask_price(pool: LiquidityPoolId, pair: TradingPair, max: Option<Price>) -> FixedI128Result {
		let price = Self::price(pair.base, pair.quote)?;
		let spread = T::LiquidityPools::ask_spread(pool, pair).ok_or(Error::<T>::NoAskSpread)?;
		let ask_price: Price = price.saturating_add(spread);

		if let Some(m) = max {
			if ask_price > m {
				return Err(Error::<T>::MarketPriceTooHigh.into());
			}
		}

		Ok(fixed_i128_from_fixed_u128(ask_price))
	}

	/// bid_price = price - bid_spread
	fn bid_price(pool: LiquidityPoolId, pair: TradingPair, min: Option<Price>) -> FixedI128Result {
		let price = Self::price(pair.base, pair.quote)?;
		let spread = T::LiquidityPools::bid_spread(pool, pair).ok_or(Error::<T>::NoBidSpread)?;
		let bid_price = price.saturating_sub(spread);

		if let Some(m) = min {
			if bid_price < m {
				return Err(Error::<T>::MarketPriceTooLow.into());
			}
		}

		Ok(fixed_i128_from_fixed_u128(bid_price))
	}

	/// usd_value = amount * price
	fn usd_value(currency_id: CurrencyId, amount: FixedI128) -> FixedI128Result {
		let price = {
			let p = Self::price(currency_id, CurrencyId::AUSD)?;
			fixed_i128_from_fixed_u128(p)
		};
		amount
			.checked_mul(&price)
			.ok_or_else(|| Error::<T>::NumOutOfBound.into())
	}
}

// Trader helpers
impl<T: Config> Module<T> {
	/// Unrealized profit and loss of a position(USD value), based on current market price.
	///
	/// unrealized_pl_of_position = (curr_price - open_price) * leveraged_held * to_usd_price
	fn unrealized_pl_of_position(position: &Position<T>) -> FixedI128Result {
		let (unrealized, _) = Self::unrealized_pl_and_market_price_of_position(position, None)?;
		Ok(unrealized)
	}

	/// Returns `Ok((unrealized_pl, market_price))` of a given position. If `price`, market price
	/// must fit this bound, else returns `None`.
	fn unrealized_pl_and_market_price_of_position(
		position: &Position<T>,
		price: Option<Price>,
	) -> result::Result<(FixedI128, FixedI128), DispatchError> {
		// open_price = abs(leveraged_debits / leveraged_held)
		let open_price = position
			.leveraged_debits
			.checked_div(&position.leveraged_held)
			.expect("ensured safe on open position")
			.saturating_abs();
		let curr_price = {
			if position.leverage.is_long() {
				Self::bid_price(position.pool, position.pair, price)?
			} else {
				Self::ask_price(position.pool, position.pair, price)?
			}
		};
		let price_delta = curr_price
			.checked_sub(&open_price)
			.expect("Non-negative integers sub can't overflow; qed");
		let unrealized = position
			.leveraged_held
			.checked_mul(&price_delta)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let usd_value = Self::usd_value(position.pair.quote, unrealized)?;

		Ok((usd_value, curr_price))
	}

	/// unrealized_pl_of_pool = pool_per_pair_long_unrealized + pool_per_pair_short_unrealized
	fn unrealized_pl_of_pool(pool_id: LiquidityPoolId) -> FixedI128Result {
		PositionsSnapshots::iter_prefix(pool_id).try_fold(FixedI128::zero(), |unrealized, (pair, pool)| {
			let long_unrealized = {
				let curr_price = Self::bid_price(pool_id, pair, None)?;
				let base_in_quote = pool
					.long
					.held
					.checked_mul(&curr_price)
					.ok_or(Error::<T>::NumOutOfBound)?;
				let profit_in_quote = base_in_quote
					.checked_add(&pool.long.debits)
					.ok_or(Error::<T>::NumOutOfBound)?;
				Self::usd_value(pair.quote, profit_in_quote)
			}?;

			let short_unrealized = {
				let curr_price = Self::ask_price(pool_id, pair, None)?;
				let base_in_quote = pool
					.short
					.held
					.checked_mul(&curr_price)
					.ok_or(Error::<T>::NumOutOfBound)?;
				let profit_in_quote = base_in_quote
					.checked_add(&pool.short.debits)
					.ok_or(Error::<T>::NumOutOfBound)?;
				Self::usd_value(pair.quote, profit_in_quote)
			}?;

			let sum = long_unrealized
				.checked_add(&short_unrealized)
				.ok_or(Error::<T>::NumOutOfBound)?;
			let new_unrealized = unrealized.checked_add(&sum).ok_or(Error::<T>::NumOutOfBound)?;
			Ok(new_unrealized)
		})
	}

	/// Unrealized profit and loss of a given trader in a pool(USD value). It is the sum of
	/// unrealized profit and loss of all positions opened by a trader.
	pub fn unrealized_pl_of_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> FixedI128Result {
		<PositionsByTrader<T>>::iter_prefix(who)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
			.filter(|p| p.pool == pool_id)
			.try_fold(FixedI128::zero(), |acc, p| {
				let unrealized = Self::unrealized_pl_of_position(&p)?;
				acc.checked_add(&unrealized)
					.ok_or_else(|| Error::<T>::NumOutOfBound.into())
			})
	}

	/// Sum of all margin held of a given trader in a pool.
	pub fn margin_held(who: &T::AccountId, pool_id: LiquidityPoolId) -> FixedI128 {
		<PositionsByTrader<T>>::iter_prefix(who)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
			.filter(|p| p.pool == pool_id)
			.fold(FixedI128::zero(), |acc, p| {
				acc.checked_add(&p.margin_held)
					.expect("margin held cannot overflow; qed")
			})
	}

	/// Accumulated swap rate of a position(USD value).
	///
	/// accumulated_swap_rate_of_position =
	///   (current_accumulated - open_accumulated) * leveraged_held
	fn accumulated_swap_rate_of_position(position: &Position<T>) -> FixedI128Result {
		let rate = T::LiquidityPools::accumulated_swap_rate(position.pool, position.pair, position.leverage.is_long())
			.checked_sub(&position.open_accumulated_swap_rate)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let accumulated_swap_rate = position
			.leveraged_debits
			.saturating_abs()
			.checked_mul(&rate)
			.ok_or(Error::<T>::NumOutOfBound)?;

		let usd_value = Self::usd_value(position.pair.quote, accumulated_swap_rate)?;
		Ok(usd_value)
	}

	/// Accumulated swap of all open positions of a given trader(USD value) in a pool.
	fn accumulated_swap_rate_of_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> FixedI128Result {
		<PositionsByTrader<T>>::iter_prefix(who)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
			.filter(|p| p.pool == pool_id)
			.try_fold(FixedI128::zero(), |acc, p| {
				let rate_of_p = Self::accumulated_swap_rate_of_position(&p)?;
				acc.checked_add(&rate_of_p)
					.ok_or_else(|| Error::<T>::NumOutOfBound.into())
			})
	}

	/// equity_of_trader = balance + unrealized_pl + accumulated_swap_rate
	pub fn equity_of_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> FixedI128Result {
		let unrealized = Self::unrealized_pl_of_trader(who, pool_id)?;
		let with_unrealized = Self::balances(who, pool_id)
			.checked_add(&unrealized)
			.ok_or(Error::<T>::NumOutOfBound)?;
		let accumulated_swap_rate = Self::accumulated_swap_rate_of_trader(who, pool_id)?;
		with_unrealized
			.checked_add(&accumulated_swap_rate)
			.ok_or_else(|| Error::<T>::NumOutOfBound.into())
	}

	/// Free margin of a given trader in a pool.
	pub fn free_margin(who: &T::AccountId, pool_id: LiquidityPoolId) -> FixedI128Result {
		let equity = Self::equity_of_trader(who, pool_id)?;
		let margin_held = Self::margin_held(who, pool_id);
		Ok(equity.saturating_sub(margin_held))
	}

	/// Margin level of a given trader in a pool.
	pub fn margin_level(who: &T::AccountId, pool_id: LiquidityPoolId) -> FixedI128Result {
		let equity = Self::equity_of_trader(who, pool_id)?;
		let leveraged_debits_in_usd = <PositionsByTrader<T>>::iter_prefix(who)
			.filter_map(|((_, position_id), _)| Self::positions(position_id))
			.filter(|p| p.pool == pool_id)
			.try_fold::<_, _, FixedI128Result>(FixedI128::zero(), |acc, p| {
				let debits_in_usd = Self::usd_value(p.pair.quote, p.leveraged_debits.saturating_abs())?;
				acc.checked_add(&debits_in_usd)
					.ok_or_else(|| Error::<T>::NumOutOfBound.into())
			})?;

		Ok(equity
			.checked_div(&leveraged_debits_in_usd)
			// if no leveraged held, margin level is max
			.unwrap_or_else(FixedI128::max_value))
	}

	/// Ensure a trader is safe.
	///
	/// Return `Ok` if ensured safe, or `Err` if not.
	fn ensure_trader_safe(who: &T::AccountId, pool_id: LiquidityPoolId, action: Action<T>) -> DispatchResult {
		let risk = Self::check_trader(who, pool_id, action)?;
		match risk {
			Risk::None => Ok(()),
			_ => Err(Error::<T>::UnsafeTrader.into()),
		}
	}

	/// Check trader risk after performing an action.
	///
	/// Return `Ok(Risk)`, or `Err` if check fails.
	fn check_trader(who: &T::AccountId, pool_id: LiquidityPoolId, action: Action<T>) -> Result<Risk, DispatchError> {
		let margin_level = Self::margin_level(who, pool_id)?;

		let new_pair_risk_threshold = match action {
			Action::OpenPosition(p) => Self::trader_risk_threshold(p.pair).unwrap_or_default(),
			_ => RiskThreshold::default(),
		};

		let trader_threshold = Self::risk_threshold_of_trader(who, pool_id);
		let risk = if margin_level <= cmp::max(trader_threshold.stop_out, new_pair_risk_threshold.stop_out).into() {
			Risk::StopOut
		} else if margin_level <= cmp::max(trader_threshold.margin_call, new_pair_risk_threshold.margin_call).into() {
			Risk::MarginCall
		} else {
			Risk::None
		};

		Ok(risk)
	}
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
enum Action<T: Config> {
	None,
	Withdraw(Balance),
	OpenPosition(Position<T>),
}

#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq)]
enum Risk {
	None,
	MarginCall,
	StopOut,
}

// Liquidity pool helpers
impl<T: Config> Module<T> {
	/// equity_of_pool = liquidity - all_unrealized_pl - all_accumulated_swap_rate
	/// In order to optimize the algorithm, ignore all_accumulated_swap_rate
	fn equity_of_pool(pool: LiquidityPoolId) -> FixedI128Result {
		let liquidity = {
			let l = <T::LiquidityPools as LiquidityPools<T::AccountId>>::liquidity(pool);
			fixed_i128_from_u128(l)
		};

		let unrealized_pl = Self::unrealized_pl_of_pool(pool)?;

		liquidity
			.checked_sub(&unrealized_pl)
			.ok_or_else(|| Error::<T>::NumOutOfBound.into())
	}

	/// Returns `(net_position, longest_leg)` of a liquidity pool.
	fn net_position_and_longest_leg(pool: LiquidityPoolId, new_position: Option<Position<T>>) -> DoubleFixedI128Result {
		PositionsSnapshots::iter_prefix(pool)
			.map(|(pair, pool)| (pair, pool))
			.chain(new_position.map_or(vec![], |p| {
				let info = if p.leverage.is_long() {
					PositionsSnapshot {
						positions_count: 1,
						long: LeveragedAmounts {
							held: p.leveraged_held,
							debits: p.leveraged_debits,
						},
						short: LeveragedAmounts {
							held: FixedI128::zero(),
							debits: FixedI128::zero(),
						},
					}
				} else {
					PositionsSnapshot {
						positions_count: 1,
						long: LeveragedAmounts {
							held: FixedI128::zero(),
							debits: FixedI128::zero(),
						},
						short: LeveragedAmounts {
							held: p.leveraged_held,
							debits: p.leveraged_debits,
						},
					}
				};
				vec![(p.pair, info)]
			}))
			.try_fold((FixedI128::zero(), FixedI128::zero()), |(net, max), (pair, pool)| {
				let new_net = pool
					.long
					.held
					.checked_add(&pool.short.held)
					.ok_or(Error::<T>::NumOutOfBound)?;
				let net_in_usd = Self::usd_value(pair.base, new_net.saturating_abs())?;

				let new_max = cmp::max(pool.long.held, pool.short.held.saturating_abs());
				let max_in_usd = Self::usd_value(pair.base, new_max.saturating_abs())?;

				let new_net = net.checked_add(&net_in_usd).ok_or(Error::<T>::NumOutOfBound)?;
				let new_max = max.checked_add(&max_in_usd).ok_or(Error::<T>::NumOutOfBound)?;
				Ok((new_net, new_max))
			})
	}

	/// ENP and ELL after performing action.
	///
	/// ENP - Equity to Net Position ratio of a liquidity pool.
	/// ELL - Equity to Longest Leg ratio of a liquidity pool.
	fn enp_and_ell_with_action(pool: LiquidityPoolId, action: Action<T>) -> DoubleFixedI128Result {
		let equity = Self::equity_of_pool(pool)?;
		let new_position = match action.clone() {
			Action::OpenPosition(p) => Some(p),
			_ => None,
		};
		let (net_position, longest_leg) = Self::net_position_and_longest_leg(pool, new_position)?;

		let equity = match action {
			Action::Withdraw(amount) => equity
				.checked_sub(&fixed_i128_from_u128(amount))
				.ok_or(Error::<T>::NumOutOfBound)?,
			_ => equity,
		};

		let enp = equity
			.checked_div(&net_position)
			// if `net_position` is zero, ENP is max
			.unwrap_or_else(FixedI128::max_value);
		let ell = equity
			.checked_div(&longest_leg)
			// if `longest_leg` is zero, ELL is max
			.unwrap_or_else(FixedI128::max_value);

		Ok((enp, ell))
	}

	/// Ensure a liquidity pool is safe after performing an action.
	///
	/// Return `Ok` if ensured safe, or `Err` if not.
	fn ensure_pool_safe(pool: LiquidityPoolId, action: Action<T>) -> DispatchResult {
		match Self::check_pool(pool, action.clone()) {
			Ok(Risk::None) => Ok(()),
			_ => match action {
				Action::None => Err(Error::<T>::UnsafePool.into()),
				_ => Err(Error::<T>::PoolWouldBeUnsafe.into()),
			},
		}
	}

	/// Check pool risk after performing an action.
	///
	/// Return `Ok(Risk)`, or `Err` if check fails.
	fn check_pool(pool_id: LiquidityPoolId, action: Action<T>) -> Result<Risk, DispatchError> {
		let (new_pair_enp_threshold, new_pair_ell_threshold) = match action.clone() {
			Action::OpenPosition(p) => (
				Self::liquidity_pool_enp_threshold(p.pair).unwrap_or_default(),
				Self::liquidity_pool_ell_threshold(p.pair).unwrap_or_default(),
			),
			_ => (RiskThreshold::default(), RiskThreshold::default()),
		};
		let (enp_threshold, ell_threshold) = Self::enp_and_ell_risk_threshold_of_pool(pool_id);

		let (enp, ell) = Self::enp_and_ell_with_action(pool_id, action)?;
		if enp <= cmp::max(enp_threshold.stop_out, new_pair_enp_threshold.stop_out).into()
			|| ell <= cmp::max(ell_threshold.stop_out, new_pair_ell_threshold.stop_out).into()
		{
			return Ok(Risk::StopOut);
		} else if enp <= cmp::max(enp_threshold.margin_call, new_pair_enp_threshold.margin_call).into()
			|| ell <= cmp::max(ell_threshold.margin_call, new_pair_ell_threshold.margin_call).into()
		{
			return Ok(Risk::MarginCall);
		}
		Ok(Risk::None)
	}

	/// Force closure position to liquidate liquidity pool based on opened positions.
	///
	/// Return `Ok` if closure success, or `Err` if not.
	fn liquidity_pool_close_position(pool: LiquidityPoolId, position_id: PositionId) -> DispatchResult {
		let position = Self::positions(position_id).ok_or(Error::<T>::PositionNotFound)?;

		let spread = {
			if position.leverage.is_long() {
				T::LiquidityPools::bid_spread(pool, position.pair)
					.ok_or(Error::<T>::NoBidSpread)
					.map(fixed_i128_from_fixed_u128)?
			} else {
				T::LiquidityPools::ask_spread(pool, position.pair)
					.ok_or(Error::<T>::NoAskSpread)
					.map(fixed_i128_from_fixed_u128)?
			}
		};

		let spread_profit = position
			.leveraged_held
			.checked_mul(&spread)
			.ok_or(Error::<T>::NumOutOfBound)?;

		let spread_profit_in_usd = Self::usd_value(position.pair.quote, spread_profit)?;
		let penalty = spread_profit_in_usd;
		let sub_amount = spread_profit_in_usd
			.checked_add(&penalty)
			.ok_or(Error::<T>::NumOutOfBound)?;

		Self::do_close_position(&position.owner, position_id, None)?;

		let realized = cmp::min(
			<T::LiquidityPools as LiquidityPools<T::AccountId>>::liquidity(position.pool),
			u128_from_fixed_i128(sub_amount),
		);
		<T::LiquidityPools as LiquidityPools<T::AccountId>>::withdraw_liquidity(
			&T::GetTreasuryAccountId::get(),
			position.pool,
			realized,
		)?;

		Ok(())
	}

	/// Return risk threshold of trader based on opened positions after performing an action.
	///
	/// Return `RiskThreshold` or `Default` value.
	fn risk_threshold_of_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> RiskThreshold {
		let (trader_margin_call, trader_stop_out) = <PositionsByTrader<T>>::iter_prefix(who)
			.filter(|((p, _), _)| *p == pool_id)
			.fold(vec![], |mut v, ((_, position_id), _)| {
				if let Some(position) = Self::positions(position_id) {
					if !v.contains(&position.pair) {
						v.push(position.pair);
					}
				}
				v
			})
			.iter()
			.filter_map(|pair| Self::trader_risk_threshold(*pair))
			.map(|v| (v.margin_call, v.stop_out))
			.fold((Permill::zero(), Permill::zero()), |max, v| {
				(cmp::max(max.0, v.0), cmp::max(max.1, v.1))
			});

		RiskThreshold {
			margin_call: trader_margin_call,
			stop_out: trader_stop_out,
		}
	}

	/// Return risk threshold of liquidity pool based on opened positions after performing an
	/// action.
	///
	/// Return `RiskThreshold` or `Default` value.
	fn enp_and_ell_risk_threshold_of_pool(pool_id: LiquidityPoolId) -> (RiskThreshold, RiskThreshold) {
		let (enp_margin_call, enp_stop_out, ell_margin_call, ell_stop_out) = PositionsSnapshots::iter_prefix(pool_id)
			.fold(vec![], |mut v, (pair, _)| {
				if !v.contains(&pair) {
					v.push(pair);
				}
				v
			})
			.iter()
			.filter_map(|pair| {
				let enp = Self::liquidity_pool_enp_threshold(*pair)?;
				let ell = Self::liquidity_pool_ell_threshold(*pair)?;
				Some((enp.margin_call, enp.stop_out, ell.margin_call, ell.stop_out))
			})
			.fold(
				(Permill::zero(), Permill::zero(), Permill::zero(), Permill::zero()),
				|max, v| {
					(
						cmp::max(max.0, v.0),
						cmp::max(max.1, v.1),
						cmp::max(max.2, v.2),
						cmp::max(max.3, v.3),
					)
				},
			);

		(
			RiskThreshold {
				margin_call: enp_margin_call,
				stop_out: enp_stop_out,
			},
			RiskThreshold {
				margin_call: ell_margin_call,
				stop_out: ell_stop_out,
			},
		)
	}

	pub fn enp_and_ell(pool: LiquidityPoolId) -> Option<(FixedI128, FixedI128)> {
		if <T::LiquidityPools as LiquidityPools<T::AccountId>>::pool_exists(pool) {
			let result = Self::enp_and_ell_with_action(pool, Action::None).ok()?;
			return Some(result);
		}
		None
	}

	/// Returns required deposit amount to make pool safe.
	pub fn pool_required_deposit(pool: LiquidityPoolId) -> Option<FixedI128> {
		let (net_position, longest_leg) = Self::net_position_and_longest_leg(pool, None).ok()?;
		let (enp_threshold, ell_threshold) = Self::enp_and_ell_risk_threshold_of_pool(pool);
		let required_equity = {
			let for_enp = net_position
				.checked_mul(&enp_threshold.margin_call.into())
				.expect("ENP margin call threshold < 1; qed");
			let for_ell = longest_leg
				.checked_mul(&ell_threshold.margin_call.into())
				.expect("ELL margin call threshold < 1; qed");
			cmp::max(for_enp, for_ell)
		};
		let equity = Self::equity_of_pool(pool).ok()?;
		let gap = required_equity.checked_sub(&equity)?;

		if gap.is_positive() {
			Some(gap)
		} else {
			Some(FixedI128::zero())
		}
	}
}

impl<T: Config> BaseLiquidityPoolManager<LiquidityPoolId, Balance> for Module<T> {
	/// Returns if `pool` has liability in margin protocol.
	fn can_remove(pool: LiquidityPoolId) -> bool {
		PositionsSnapshots::iter_prefix(pool).fold(0, |num, (_, snapshot)| num + snapshot.positions_count) == 0
	}

	fn ensure_can_withdraw(pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		Self::ensure_pool_safe(pool_id, Action::Withdraw(amount))
	}
}

impl<T: Config> MarginProtocolLiquidityPoolsManager for Module<T> {
	fn ensure_can_enable_trading_pair(pool_id: LiquidityPoolId, pair: TradingPair) -> DispatchResult {
		Self::trader_risk_threshold(pair).ok_or(Error::<T>::NoRiskThreshold)?;
		let enp_threshold = Self::liquidity_pool_enp_threshold(pair).ok_or(Error::<T>::NoRiskThreshold)?;
		let ell_threshold = Self::liquidity_pool_ell_threshold(pair).ok_or(Error::<T>::NoRiskThreshold)?;

		let (enp, ell) = Self::enp_and_ell_with_action(pool_id, Action::None)?;
		if enp <= enp_threshold.stop_out.into()
			|| ell <= ell_threshold.stop_out.into()
			|| enp <= enp_threshold.margin_call.into()
			|| ell <= ell_threshold.margin_call.into()
		{
			return Err(Error::<T>::PoolWouldBeUnsafe.into());
		}
		Ok(())
	}
}

/// Error which may occur while executing the off-chain code.
#[cfg_attr(test, derive(PartialEq))]
enum OffchainErr {
	OffchainLock,
	SubmitTransaction,
	NotValidator,
	CheckFail,
}

// constant for offchain worker
const LOCK_DURATION: u64 = 40_000; // 40 sec
const OFFCHAIN_WORKER_LOCK: &[u8] = b"laminar/margin-protocol/offchain-worker-lock";
#[cfg(feature = "std")]
const TAG: &str = "MARGIN_PROTOCOL_OFFCHAIN_WORKER";

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::OffchainLock => write!(fmt, "Failed to get or extend lock"),
			OffchainErr::SubmitTransaction => write!(fmt, "Failed to submit transaction"),
			OffchainErr::NotValidator => write!(fmt, "Not validator"),
			OffchainErr::CheckFail => write!(fmt, "Check fail"),
		}
	}
}

impl<T: Config> Module<T> {
	/// Get a list of `(trader, pool_id)`
	fn get_traders() -> Vec<(T::AccountId, LiquidityPoolId)> {
		// TODO: use key iter after this gets closed https://github.com/paritytech/substrate/issues/5319
		let mut traders: Vec<(T::AccountId, LiquidityPoolId)> =
			<Positions<T>>::iter().map(|(_, p)| (p.owner, p.pool)).collect();
		traders.sort();
		traders.dedup(); // dedup works as unique for sorted vec, so we sort first
		traders
	}

	/// Get a list of pools
	fn get_pools() -> Vec<LiquidityPoolId> {
		// TODO: use key iter after this gets closed https://github.com/paritytech/substrate/issues/5319
		let mut pools: Vec<LiquidityPoolId> = <Positions<T>>::iter().map(|(_, p)| p.pool).collect();
		#[allow(clippy::stable_sort_primitive)] // need stable sort to be deterministic
		pools.sort();
		pools.dedup(); // dedup works as unique for sorted vec, so we sort first
		pools
	}

	#[allow(unused_variables)] // `block_number` is used in macros
	fn offchain_worker(block_number: T::BlockNumber) -> Result<(), OffchainErr> {
		// check if we are a potential validator
		if !sp_io::offchain::is_validator() {
			return Err(OffchainErr::NotValidator);
		}

		// Acquire offchain worker lock.
		let lock_expiration = Duration::from_millis(LOCK_DURATION);
		let mut lock = StorageLock::<'_, Time>::with_deadline(&OFFCHAIN_WORKER_LOCK, lock_expiration);
		let mut guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

		debug::native::trace!(target: TAG, "Started [block_number = {:?}]", block_number);

		for (trader, pool_id) in Self::get_traders() {
			match Self::check_trader(&trader, pool_id, Action::None).map_err(|_| OffchainErr::CheckFail)? {
				Risk::StopOut => {
					let who = T::Lookup::unlookup(trader.clone());
					let call = Call::<T>::trader_stop_out(who, pool_id);
					SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
						.map_err(|_| OffchainErr::SubmitTransaction)?;
					debug::native::trace!(
						target: TAG,
						"Trader liquidate [trader = {:?}, block_number = {:?}]",
						trader,
						block_number
					);
				}
				Risk::MarginCall => {
					if !Self::is_trader_margin_called(&trader, pool_id) {
						let who = T::Lookup::unlookup(trader.clone());
						let call = Call::<T>::trader_margin_call(who, pool_id);
						SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
							.map_err(|_| OffchainErr::SubmitTransaction)?;
						debug::native::trace!(
							target: TAG,
							"Trader margin call [trader = {:?}, block_number = {:?}]",
							trader,
							block_number
						);
					}
				}
				Risk::None => {
					if Self::is_trader_margin_called(&trader, pool_id) {
						let who = T::Lookup::unlookup(trader.clone());
						let call = Call::<T>::trader_become_safe(who, pool_id);
						SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
							.map_err(|_| OffchainErr::SubmitTransaction)?;
						debug::native::trace!(
							target: TAG,
							"Trader become safe [trader = {:?}, block_number = {:?}]",
							trader,
							block_number
						);
					}
				}
			}

			guard.extend_lock().map_err(|_| OffchainErr::OffchainLock)?;
		}

		for pool_id in Self::get_pools() {
			match Self::check_pool(pool_id, Action::None).map_err(|_| OffchainErr::CheckFail)? {
				Risk::StopOut => {
					let call = Call::<T>::liquidity_pool_force_close(pool_id);
					SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
						.map_err(|_| OffchainErr::SubmitTransaction)?;
					debug::native::trace!(
						target: TAG,
						"Liquidity pool liquidate [pool_id = {:?}, block_number = {:?}]",
						pool_id,
						block_number
					);
				}
				Risk::MarginCall => {
					if !Self::is_pool_margin_called(&pool_id) {
						let call = Call::<T>::liquidity_pool_margin_call(pool_id);
						SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
							.map_err(|_| OffchainErr::SubmitTransaction)?;
						debug::native::trace!(
							target: TAG,
							"Liquidity pool margin call [pool_id = {:?}, block_number = {:?}]",
							pool_id,
							block_number
						);
					}
				}
				Risk::None => {
					if Self::is_pool_margin_called(&pool_id) {
						let call = Call::<T>::liquidity_pool_become_safe(pool_id);
						SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
							.map_err(|_| OffchainErr::SubmitTransaction)?;
						debug::native::trace!(
							target: TAG,
							"Liquidity pool become safe [pool_id = {:?}, block_number = {:?}]",
							pool_id,
							block_number
						);
					}
				}
			}

			guard.extend_lock().map_err(|_| OffchainErr::OffchainLock)?;
		}

		debug::native::trace!(target: TAG, "Finished [block_number = {:?}]", block_number);
		Ok(())

		// drop `guard` and unlock implicitly at end of scope.
	}

	fn is_trader_margin_called(who: &T::AccountId, pool_id: LiquidityPoolId) -> bool {
		<MarginCalledTraders<T>>::contains_key(&who, pool_id)
	}

	fn is_pool_margin_called(pool_id: &LiquidityPoolId) -> bool {
		MarginCalledPools::contains_key(pool_id)
	}

	fn should_stop_out_trader(who: &T::AccountId, pool_id: LiquidityPoolId) -> Result<bool, OffchainErr> {
		match Self::check_trader(who, pool_id, Action::None).map_err(|_| OffchainErr::CheckFail)? {
			Risk::StopOut => Ok(true),
			_ => Ok(false),
		}
	}

	fn should_liquidate_pool(pool_id: LiquidityPoolId) -> Result<bool, OffchainErr> {
		match Self::check_pool(pool_id, Action::None).map_err(|_| OffchainErr::CheckFail)? {
			Risk::StopOut => Ok(true),
			_ => Ok(false),
		}
	}
}

impl<T: Config> frame_support::unsigned::ValidateUnsigned for Module<T> {
	type Call = Call<T>;

	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		match call {
			Call::trader_margin_call(who, pool_id) => {
				let trader = T::Lookup::lookup(who.clone())
					.map_err(|_| TransactionValidityError::from(InvalidTransaction::Stale))?;
				if Self::is_trader_margin_called(&trader, *pool_id) {
					return InvalidTransaction::Stale.into();
				}

				ValidTransaction::with_tag_prefix("margin_protocol/trader_margin_call")
					.priority(T::UnsignedPriority::get())
					.and_provides((who, pool_id))
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			Call::trader_become_safe(who, pool_id) => {
				let trader = T::Lookup::lookup(who.clone())
					.map_err(|_| TransactionValidityError::from(InvalidTransaction::Stale))?;
				if !Self::is_trader_margin_called(&trader, *pool_id) {
					return InvalidTransaction::Stale.into();
				}

				ValidTransaction::with_tag_prefix("margin_protocol/trader_become_safe")
					.priority(T::UnsignedPriority::get())
					.and_provides((who, pool_id))
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			Call::trader_stop_out(who, pool_id) => {
				let trader = T::Lookup::lookup(who.clone())
					.map_err(|_| TransactionValidityError::from(InvalidTransaction::Stale))?;
				if Self::should_stop_out_trader(&trader, *pool_id).ok() == Some(true) {
					return ValidTransaction::with_tag_prefix("margin_protocol/trader_stop_out")
						.priority(T::UnsignedPriority::get())
						.and_provides((who, pool_id))
						.longevity(64_u64)
						.propagate(true)
						.build();
				}
				InvalidTransaction::Stale.into()
			}
			Call::liquidity_pool_margin_call(pool_id) => {
				if Self::is_pool_margin_called(pool_id) {
					return InvalidTransaction::Stale.into();
				}
				ValidTransaction::with_tag_prefix("margin_protocol/liquidity_pool_margin_call")
					.priority(T::UnsignedPriority::get())
					.and_provides(pool_id)
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			Call::liquidity_pool_become_safe(pool_id) => {
				if !Self::is_pool_margin_called(pool_id) {
					return InvalidTransaction::Stale.into();
				}

				ValidTransaction::with_tag_prefix("margin_protocol/liquidity_pool_become_safe")
					.priority(T::UnsignedPriority::get())
					.and_provides(pool_id)
					.longevity(64_u64)
					.propagate(true)
					.build()
			}
			Call::liquidity_pool_force_close(pool_id) => {
				if Self::should_liquidate_pool(*pool_id).ok() == Some(true) {
					return ValidTransaction::with_tag_prefix("margin_protocol/liquidity_pool_force_close")
						.priority(T::UnsignedPriority::get())
						.and_provides(pool_id)
						.longevity(64_u64)
						.propagate(true)
						.build();
				}

				InvalidTransaction::Stale.into()
			}
			_ => InvalidTransaction::Call.into(),
		}
	}
}
