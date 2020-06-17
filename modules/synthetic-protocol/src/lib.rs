#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, weights::DispatchClass};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedSub, Saturating, Zero},
	DispatchError, DispatchResult, FixedPointNumber, FixedU128,
};
use sp_std::result;
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_signed};

use orml_prices::Price;
use orml_traits::{BasicCurrency, MultiCurrency, PriceProvider};

use module_primitives::{Balance, CurrencyId, LiquidityPoolId};
use module_traits::{LiquidityPools, SyntheticProtocolLiquidityPools};

mod mock;
mod tests;

pub trait Trait: module_synthetic_tokens::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The `MultiCurrency` implementation for synthetic.
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;

	/// Collateral currency.
	type CollateralCurrency: BasicCurrency<Self::AccountId, Balance = Balance>;

	/// `Get` implementation of collateral currency ID.
	type GetCollateralCurrencyId: Get<CurrencyId>;

	/// Provides market prices.
	type PriceProvider: PriceProvider<CurrencyId, Price>;

	/// The basic liquidity pools.
	type LiquidityPools: LiquidityPools<Self::AccountId>;

	/// The synthetic protocol liquidity pools.
	type SyntheticProtocolLiquidityPools: SyntheticProtocolLiquidityPools<Self::AccountId>;
}

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticProtocol {}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
	{
		/// Synthetic token minted: (who, synthetic_currency_id, pool_id, collateral_amount, synthetic_amount)
		Minted(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),

		/// Synthetic token redeemed: (who, synthetic_currency_id, pool_id, collateral_amount, synthetic_amount)
		Redeemed(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),

		/// Synthetic token liquidated: (who, synthetic_currency_id, pool_id, collateral_amount, synthetic_amount)
		Liquidated(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),

		/// Collateral added: (who, synthetic_currency_id, pool_id, collateral_amount)
		CollateralAdded(AccountId, CurrencyId, LiquidityPoolId, Balance),

		/// Collateral withdrew: (who, synthetic_currency_id, pool_id, collateral_amount)
		CollateralWithdrew(AccountId, CurrencyId, LiquidityPoolId, Balance),
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const GetCollateralCurrencyId: CurrencyId = T::GetCollateralCurrencyId::get();

		/// Mint synthetic tokens.
		#[weight = 10_000]
		pub fn mint(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			currency_id: CurrencyId,
			#[compact] collateral_amount: Balance,
			max_price: Price,
		) {
			let who = ensure_signed(origin)?;
			let synthetic_amount = Self::_mint(&who, pool_id, currency_id, collateral_amount, max_price)?;

			Self::deposit_event(RawEvent::Minted(who, currency_id, pool_id, collateral_amount, synthetic_amount));
		}

		/// Redeem collateral.
		#[weight = 10_000]
		pub fn redeem(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			currency_id: CurrencyId,
			#[compact] synthetic_amount: Balance,
			min_price: Price,
		) {
			let who = ensure_signed(origin)?;
			let collateral_amount = Self::_redeem(&who, pool_id, currency_id, synthetic_amount, min_price)?;

			Self::deposit_event(RawEvent::Redeemed(who, currency_id, pool_id, collateral_amount, synthetic_amount));
		}

		/// Liquidite `currency_id` in `pool_id` by `synthetic_amount`.
		#[weight = (20_000, DispatchClass::Operational)]
		pub fn liquidate(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			currency_id: CurrencyId,
			#[compact] synthetic_amount: Balance,
		) {
			let who = ensure_signed(origin)?;
			let collateral_amount = Self::_liquidate(&who, pool_id, currency_id, synthetic_amount)?;

			Self::deposit_event(RawEvent::Liquidated(who, currency_id, pool_id, collateral_amount, synthetic_amount));
		}

		/// Add collateral to `currency_id` in `pool_id` by `collateral_amount`.
		#[weight = (10_000, DispatchClass::Operational)]
		pub fn add_collateral(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			currency_id: CurrencyId,
			#[compact] collateral_amount: Balance,
		) {
			let who = ensure_signed(origin)?;
			Self::_add_collateral(&who, pool_id, currency_id, collateral_amount)?;

			Self::deposit_event(RawEvent::CollateralAdded(who, currency_id, pool_id, collateral_amount));
		}

		/// Withdraw all available collateral.
		///
		/// May only be called from the pool owner.
		#[weight = 10_000]
		pub fn withdraw_collateral(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			currency_id: CurrencyId,
		) {
			let who = ensure_signed(origin)?;
			let withdrew_collateral_amount = Self::_withdraw_collateral(&who, pool_id, currency_id)?;

			Self::deposit_event(RawEvent::CollateralWithdrew(who, currency_id, pool_id, withdrew_collateral_amount));
		}
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Insufficient liquidity in pool for minting.
		InsufficientLiquidityInPool,

		/// Required synthetic minting is not supported by pool.
		CannotMintInPool,

		/// Ask price is too high.
		AskPriceTooHigh,

		/// Bind price is too low.
		BidPriceTooLow,

		/// Number overflow in calculation.
		NumOverflow,

		/// No price from provider.
		NoPrice,

		/// Negative required additional amount from pool.
		///
		/// May caused by wrong spread and ratio config of pool
		NegativeAdditionalCollateralAmount,

		/// Insufficient amount of synthetic in the position for liquidation or redeeming.
		InsufficientSyntheticInPosition,

		/// Insufficient amount of collateral in the position for liquidation or redeeming.
		InsufficientCollateralInPosition,

		/// Insufficient amount of collateral locked in protocol.
		InsufficientLockedCollateral,

		/// Still in safe position and cannot be liquidated.
		StillInSafePosition,

		/// Caller doesn't have permission.
		NoPermission,

		/// Bid spread not set.
		NoBidSpread,

		/// Ask spread not set.
		NoAskSpread,

		/// The currency is not enabled in synthetic protocol.
		NotValidSyntheticCurrencyId,
	}
}

// Dispatch calls

type SyntheticTokens<T> = module_synthetic_tokens::Module<T>;
type BalanceResult = result::Result<Balance, DispatchError>;

impl<T: Trait> Module<T> {
	fn _mint(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		collateral: Balance,
		max_price: Price,
	) -> BalanceResult {
		ensure!(
			T::SyntheticCurrencyIds::get().contains(&currency_id),
			Error::<T>::NotValidSyntheticCurrencyId
		);

		ensure!(
			T::SyntheticProtocolLiquidityPools::can_mint(pool_id, currency_id),
			Error::<T>::CannotMintInPool
		);

		T::CollateralCurrency::ensure_can_withdraw(who, collateral)?;

		let price =
			T::PriceProvider::get_price(currency_id, T::GetCollateralCurrencyId::get()).ok_or(Error::<T>::NoPrice)?;
		let ask_price = Self::_ask_price(pool_id, currency_id, price, max_price)?;

		// synthetic = collateral / ask_price
		let synthetic = Price::from_inner(collateral)
			.checked_div(&ask_price)
			.map(|x| x.into_inner())
			.ok_or(Error::<T>::NumOverflow)?;

		// synthetic_value = synthetic * price
		// `synthetic_value` is how much `synthetic` values in collateral unit.
		let synthetic_value = price.checked_mul_int(synthetic).ok_or(Error::<T>::NumOverflow)?;
		// additional_collateral = synthetic_value * (1 + ratio) - collateral
		let additional_collateral =
			Self::_additional_collateral_amount(pool_id, currency_id, collateral, synthetic_value)?;

		ensure!(
			T::LiquidityPools::liquidity(pool_id) >= additional_collateral,
			Error::<T>::InsufficientLiquidityInPool,
		);

		// mint synthetic
		T::MultiCurrency::deposit(currency_id, who, synthetic)?;

		// collateralise
		T::CollateralCurrency::transfer(who, &<SyntheticTokens<T>>::account_id(), collateral)
			.expect("ensured enough balance of sender; qed");
		T::LiquidityPools::withdraw_liquidity(&<SyntheticTokens<T>>::account_id(), pool_id, additional_collateral)
			.expect("ensured enough collateral in liquidity pool; qed");

		let total_collateral = collateral + additional_collateral;

		<SyntheticTokens<T>>::add_position(pool_id, currency_id, total_collateral, synthetic);

		Ok(synthetic)
	}

	fn _redeem(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		synthetic: Balance,
		min_price: Price,
	) -> BalanceResult {
		ensure!(
			T::SyntheticCurrencyIds::get().contains(&currency_id),
			Error::<T>::NotValidSyntheticCurrencyId
		);

		T::MultiCurrency::ensure_can_withdraw(currency_id, who, synthetic)?;

		let price =
			T::PriceProvider::get_price(currency_id, T::GetCollateralCurrencyId::get()).ok_or(Error::<T>::NoPrice)?;
		// bid_price = price - bid_spread
		let bid_price = Self::_bid_price(pool_id, currency_id, price, Some(min_price))?;

		// collateral = synthetic * bid_price
		let redeemed_collateral = bid_price.checked_mul_int(synthetic).ok_or(Error::<T>::NumOverflow)?;

		let (collateral_position_delta, pool_refund_collateral) =
			Self::_collateral_change_on_remove_position(pool_id, currency_id, price, synthetic, redeemed_collateral)?;

		T::CollateralCurrency::ensure_can_withdraw(
			&<SyntheticTokens<T>>::account_id(),
			redeemed_collateral + pool_refund_collateral,
		)
		.map_err(|_| Error::<T>::InsufficientLockedCollateral)?;

		// burn synthetic
		T::MultiCurrency::withdraw(currency_id, who, synthetic)?;

		// redeem collateral
		T::CollateralCurrency::transfer(&<SyntheticTokens<T>>::account_id(), who, redeemed_collateral)
			.expect("ensured enough locked collateral; qed");
		T::LiquidityPools::deposit_liquidity(&<SyntheticTokens<T>>::account_id(), pool_id, pool_refund_collateral)
			.expect("ensured enough locked collateral; qed");

		<SyntheticTokens<T>>::remove_position(pool_id, currency_id, collateral_position_delta, synthetic);

		Ok(redeemed_collateral)
	}

	fn _liquidate(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		synthetic: Balance,
	) -> BalanceResult {
		ensure!(
			T::SyntheticCurrencyIds::get().contains(&currency_id),
			Error::<T>::NotValidSyntheticCurrencyId
		);

		T::MultiCurrency::ensure_can_withdraw(currency_id, who, synthetic)?;

		let price =
			T::PriceProvider::get_price(currency_id, T::GetCollateralCurrencyId::get()).ok_or(Error::<T>::NoPrice)?;
		let bid_price = Self::_bid_price(pool_id, currency_id, price, None)?;
		// collateral = synthetic * bid_price
		let collateral = bid_price.checked_mul_int(synthetic).ok_or(Error::<T>::NumOverflow)?;

		let (collateral_position_delta, pool_refund_collateral, incentive) =
			Self::_collateral_change_on_liquidation(pool_id, currency_id, price, synthetic, collateral)?;

		// burn synthetic
		T::MultiCurrency::withdraw(currency_id, who, synthetic)?;

		// Give liquidator collateral and incentive.
		let collateral_with_incentive = collateral.checked_add(incentive).ok_or(Error::<T>::NumOverflow)?;
		T::CollateralCurrency::transfer(&<SyntheticTokens<T>>::account_id(), who, collateral_with_incentive)
			.expect("ensured enough locked collateral; qed");

		// refund to pool
		T::LiquidityPools::deposit_liquidity(&<SyntheticTokens<T>>::account_id(), pool_id, pool_refund_collateral)
			.expect("ensured enough locked collateral; qed");

		<SyntheticTokens<T>>::remove_position(pool_id, currency_id, collateral_position_delta, synthetic);

		Ok(collateral)
	}

	fn _add_collateral(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		collateral: Balance,
	) -> DispatchResult {
		ensure!(
			T::SyntheticCurrencyIds::get().contains(&currency_id),
			Error::<T>::NotValidSyntheticCurrencyId
		);

		T::CollateralCurrency::ensure_can_withdraw(who, collateral)?;

		T::LiquidityPools::deposit_liquidity(who, pool_id, collateral).expect("ensured enough balance; qed");
		T::LiquidityPools::withdraw_liquidity(&<SyntheticTokens<T>>::account_id(), pool_id, collateral)
			.expect("have deposited equal amount; qed");

		<SyntheticTokens<T>>::add_position(pool_id, currency_id, collateral, Zero::zero());

		Ok(())
	}

	fn _withdraw_collateral(who: &T::AccountId, pool_id: LiquidityPoolId, currency_id: CurrencyId) -> BalanceResult {
		ensure!(
			T::SyntheticCurrencyIds::get().contains(&currency_id),
			Error::<T>::NotValidSyntheticCurrencyId
		);

		ensure!(T::LiquidityPools::is_owner(pool_id, who), Error::<T>::NoPermission);

		let price =
			T::PriceProvider::get_price(currency_id, T::GetCollateralCurrencyId::get()).ok_or(Error::<T>::NoPrice)?;
		let (collateral_position_delta, pool_refund_collateral) =
			Self::_collateral_change_on_remove_position(pool_id, currency_id, price, Zero::zero(), Zero::zero())?;

		T::CollateralCurrency::ensure_can_withdraw(&<SyntheticTokens<T>>::account_id(), pool_refund_collateral)
			.map_err(|_| Error::<T>::InsufficientLockedCollateral)?;

		T::LiquidityPools::deposit_liquidity(&<SyntheticTokens<T>>::account_id(), pool_id, pool_refund_collateral)
			.expect("ensured enough locked collateral; qed");
		T::LiquidityPools::withdraw_liquidity(who, pool_id, pool_refund_collateral)
			.expect("have deposited equal amount; qed");

		<SyntheticTokens<T>>::remove_position(pool_id, currency_id, collateral_position_delta, Zero::zero());

		Ok(pool_refund_collateral)
	}
}

// other private methods

impl<T: Trait> Module<T> {
	/// Get ask price from liquidity pool for a given currency. Would fail if price could not meet max slippage.
	///
	/// ask_price = price + ask_spread
	fn _ask_price(
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		price: Price,
		max_price: Price,
	) -> result::Result<Price, DispatchError> {
		let ask_spread =
			T::SyntheticProtocolLiquidityPools::ask_spread(pool_id, currency_id).ok_or(Error::<T>::NoAskSpread)?;
		let ask_price = price
			.checked_add(&Price::from_inner(ask_spread))
			.ok_or(Error::<T>::NumOverflow)?;

		ensure!(ask_price <= max_price, Error::<T>::AskPriceTooHigh);
		Ok(ask_price)
	}

	/// Get bid price from liquidity pool for a given currency. Would fail if price could not meet max slippage.
	///
	/// bid_price = price - bid_spread
	fn _bid_price(
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		price: Price,
		min_price: Option<Price>,
	) -> result::Result<Price, DispatchError> {
		let bid_spread =
			T::SyntheticProtocolLiquidityPools::bid_spread(pool_id, currency_id).ok_or(Error::<T>::NoBidSpread)?;
		let bid_price = price
			.checked_sub(&Price::from_inner(bid_spread))
			.expect("price > spread_amount; qed");

		if let Some(min) = min_price {
			ensure!(bid_price >= min, Error::<T>::BidPriceTooLow);
		}
		Ok(bid_price)
	}

	/// Calculate liquidity provider's collateral parts:
	///
	/// synthetic_value * (1 + ratio) - collateral
	fn _additional_collateral_amount(
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		collateral: Balance,
		synthetic_value: Balance,
	) -> BalanceResult {
		let with_additional_collateral = Self::_with_additional_collateral(pool_id, currency_id, synthetic_value)?;

		// Cannot overflow as long as `ratio` bigger than `ask_spread`, not likely to happen with reasonable config in pool,
		// but better to be safe than sorry.
		with_additional_collateral
			.checked_sub(collateral)
			.ok_or(Error::<T>::NegativeAdditionalCollateralAmount.into())
	}

	/// Returns `collateral * (1 + ratio)`.
	fn _with_additional_collateral(
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		collateral: Balance,
	) -> BalanceResult {
		let ratio = T::SyntheticProtocolLiquidityPools::additional_collateral_ratio(pool_id, currency_id);
		let additional = ratio * collateral;

		collateral.checked_add(additional).ok_or(Error::<T>::NumOverflow.into())
	}

	/// Calculate position change for a remove, if ok, return with `(collateral_position_delta, pool_refund_collateral)`
	fn _collateral_change_on_remove_position(
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		price: Price,
		burned_synthetic: Balance,
		redeemed_collateral: Balance,
	) -> result::Result<(Balance, Balance), DispatchError> {
		let (collateral_position, synthetic_position) = <SyntheticTokens<T>>::get_position(pool_id, currency_id);

		ensure!(
			synthetic_position >= burned_synthetic,
			Error::<T>::InsufficientSyntheticInPosition,
		);
		let new_synthetic_position = synthetic_position
			.checked_sub(burned_synthetic)
			.expect("ensured high enough synthetic position; qed");

		// new_synthetic_value = new_synthetic_position * price
		let new_synthetic_value = price
			.checked_mul_int(new_synthetic_position)
			.ok_or(Error::<T>::NumOverflow)?;
		let required_collateral = Self::_with_additional_collateral(pool_id, currency_id, new_synthetic_value)?;

		let mut collateral_position_delta = redeemed_collateral;
		let mut pool_refund_collateral = Zero::zero();
		if required_collateral <= collateral_position {
			// collateral_position_delta = collateral_position - required_collateral
			collateral_position_delta = collateral_position
				.checked_sub(required_collateral)
				.expect("ensured high enough collateral position; qed");

			// pool_refund_collateral = collateral_position_delta - collateral
			pool_refund_collateral = collateral_position_delta
				.checked_sub(redeemed_collateral)
				.ok_or(Error::<T>::InsufficientCollateralInPosition)?;
		}

		Ok((collateral_position_delta, pool_refund_collateral))
	}

	/// Calculate position change and incentive for a remove.
	///
	/// If `Ok`, return with `(collateral_position_delta, pool_refund_collateral, incentive)`
	fn _collateral_change_on_liquidation(
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		price: Price,
		burned_synthetic: Balance,
		liquidized_collateral: Balance,
	) -> result::Result<(Balance, Balance, Balance), DispatchError> {
		let (collateral_position, synthetic_position) = <SyntheticTokens<T>>::get_position(pool_id, currency_id);
		ensure!(
			synthetic_position >= burned_synthetic,
			Error::<T>::InsufficientSyntheticInPosition,
		);
		ensure!(
			collateral_position >= liquidized_collateral,
			Error::<T>::InsufficientCollateralInPosition,
		);

		// synthetic_position_value = synthetic_position * price
		let synthetic_position_value = price
			.checked_mul_int(synthetic_position)
			.ok_or(Error::<T>::NumOverflow)?;
		// if synthetic position not backed by enough collateral, no incentive
		if collateral_position <= synthetic_position_value {
			return Ok((liquidized_collateral, Zero::zero(), Zero::zero()));
		}

		// current_ratio = collateral_position / synthetic_position_value
		let current_ratio =
			FixedU128::checked_from_rational(collateral_position, synthetic_position_value).unwrap_or_default();

		// in safe position if ratio > liquidation_ratio
		ensure!(
			!Self::is_safe_collateral_ratio(currency_id, current_ratio),
			Error::<T>::StillInSafePosition
		);

		let new_synthetic_position = synthetic_position
			.checked_sub(burned_synthetic)
			.expect("ensured high enough synthetic position; qed");
		let new_collateral_position = collateral_position
			.checked_sub(liquidized_collateral)
			.expect("ensured high enough collateral position; qed");

		// with_current_ratio = new_synthetic_position * price * current_ratio
		let with_current_ratio = {
			let new_synthetic_position_value = price
				.checked_mul_int(new_synthetic_position)
				.ok_or(Error::<T>::NumOverflow)?;
			let with_current_ratio = current_ratio
				.checked_mul_int(new_synthetic_position_value)
				.ok_or(Error::<T>::NumOverflow)?;
			with_current_ratio
		};

		if new_collateral_position > with_current_ratio {
			// available_for_incentive = new_collateral_position - with_current_ratio
			let available_for_incentive = new_collateral_position
				.checked_sub(with_current_ratio)
				.expect("ensured new collateral position higher; qed");
			let incentive_ratio = <SyntheticTokens<T>>::incentive_ratio(currency_id, current_ratio);
			// incentive = available_for_incentive * incentive_ratio
			let incentive = incentive_ratio
				.checked_mul_int(available_for_incentive)
				.ok_or(Error::<T>::NumOverflow)?;

			let pool_refund_collateral = available_for_incentive
				.checked_sub(incentive)
				.expect("available_for_incentive > incentive; qed");
			let collateral_with_incentive = liquidized_collateral
				.checked_add(incentive)
				.ok_or(Error::<T>::NumOverflow)?;
			let collateral_with_incentive_and_refund = collateral_with_incentive
				.checked_add(pool_refund_collateral)
				.ok_or(Error::<T>::NumOverflow)?;
			Ok((collateral_with_incentive_and_refund, pool_refund_collateral, incentive))
		} else {
			// no more incentive could be given
			Ok((liquidized_collateral, Zero::zero(), Zero::zero()))
		}
	}
}

// RPC methods.
impl<T: Trait> Module<T> {
	/// Collateral ratio of the `currency_id` in `pool_id`.
	///
	/// collateral_ratio = collateral_position / (synthetic_position * price)
	pub fn collateral_ratio(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<FixedU128> {
		let (collateral_position, synthetic_position) = <SyntheticTokens<T>>::get_position(pool_id, currency_id);
		let price = T::PriceProvider::get_price(currency_id, T::GetCollateralCurrencyId::get())?;
		let synthetic_position_value = price.checked_mul_int(synthetic_position)?;

		Some(FixedU128::checked_from_rational(collateral_position, synthetic_position_value).unwrap_or_default())
	}

	/// Check if a given collateral `ratio` of `currency_id` is safe or not.
	pub fn is_safe_collateral_ratio(currency_id: CurrencyId, ratio: FixedU128) -> bool {
		let liquidation_ratio = <SyntheticTokens<T>>::liquidation_ratio_or_default(currency_id);
		let safe_ratio_threshold = Into::<FixedU128>::into(liquidation_ratio).saturating_add(FixedU128::one());
		ratio > safe_ratio_threshold
	}
}
