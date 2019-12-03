#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use rstd::{convert::TryInto, result};
use sp_runtime::{
	traits::{CheckedAdd, CheckedMul, CheckedSub, Convert, Saturating, Zero},
	Permill,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_signed};

use orml_prices::Price;
use orml_traits::{BasicCurrency, MultiCurrency, PriceProvider};
use orml_utilities::FixedU128;

use traits::{LiquidityPoolsConfig, LiquidityPoolsCurrency};

mod mock;
mod tests;

type ErrorOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::Error;

pub trait Trait: synthetic_tokens::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Self::Balance, CurrencyId = Self::CurrencyId>;
	type CollateralCurrency: BasicCurrency<Self::AccountId, Balance = Self::Balance, Error = ErrorOf<Self>>;
	type GetCollateralCurrencyId: Get<Self::CurrencyId>;
	type PriceProvider: PriceProvider<Self::CurrencyId, Price>;
	type LiquidityPoolsConfig: LiquidityPoolsConfig<
		CurrencyId = Self::CurrencyId,
		LiquidityPoolId = Self::LiquidityPoolId,
	>;
	type LiquidityPoolsCurrency: LiquidityPoolsCurrency<
		Self::AccountId,
		CurrencyId = Self::CurrencyId,
		LiquidityPoolId = Self::LiquidityPoolId,
		Balance = Self::Balance,
	>;
	type BalanceToPrice: Convert<Self::Balance, Price>;
	type PriceToBalance: Convert<Price, Self::Balance>;
}

const _MAX_SPREAD: Permill = Permill::from_percent(3); // TODO: set this

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticProtocol {}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		CurrencyId = <T as synthetic_tokens::Trait>::CurrencyId,
		Balance = <T as synthetic_tokens::Trait>::Balance,
		LiquidityPoolId = <T as synthetic_tokens::Trait>::LiquidityPoolId,
	{
		/// Synthetic token minted.
		/// (who, synthetic_token_id, liquidity_pool_id, collateral_amount, synthetic_amount)
		Minted(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),
		/// Synthetic token redeemed.
		/// (who, synthetic_token_id, liquidity_pool_id, collateral_amount, synthetic_amount)
		Redeemed(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),
		/// Synthetic token liquidated.
		/// (who, synthetic_token_id, liquidity_pool_id, collateral_amount, synthetic_amount)
		Liquidated(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn mint(
			origin,
			pool_id: T::LiquidityPoolId,
			currency_id: T::CurrencyId,
			collateral_amount: T::Balance,
			max_slippage: Permill
		) {
			let who = ensure_signed(origin)?;
			let synthetic_amount = Self::_mint(&who, pool_id, currency_id, collateral_amount, max_slippage)?;

			Self::deposit_event(RawEvent::Minted(who, currency_id, pool_id, collateral_amount, synthetic_amount));
		}

		pub fn redeem(
			origin,
			pool_id: T::LiquidityPoolId,
			currency_id: T::CurrencyId,
			synthetic_amount: T::Balance,
			max_slippage: Permill,
		) {
			let who = ensure_signed(origin)?;
			let collateral_amount = Self::_redeem(&who, pool_id, currency_id, synthetic_amount, max_slippage)?;

			Self::deposit_event(RawEvent::Redeemed(who, currency_id, pool_id, collateral_amount, synthetic_amount));
		}

		pub fn liquidate(
			origin,
			pool_id: T::LiquidityPoolId,
			currency_id: T::CurrencyId,
			synthetic_amount: T::Balance,
		) {
			let who = ensure_signed(origin)?;
			let collateral_amount = Self::_liquidate(&who, pool_id, currency_id, synthetic_amount)?;

			Self::deposit_event(RawEvent::Liquidated(who, currency_id, pool_id, collateral_amount, synthetic_amount));
		}
	}
}

decl_error! {
	pub enum Error
	{
		BalanceTooLow,
		LiquidityProviderBalanceTooLow,
		SlippageTooHigh,
		NumOverflow,
		NoPrice,
		NegativeAdditionalCollateralAmount,
		LiquidityPoolSyntheticPositionTooLow,
		LiquidityPoolCollateralPositionTooLow,
		NotEnoughLockedCollateralAvailable,
		StillInSafePosition,
		BalanceToU128Failed,
	}
}

// Dispatch calls

type SyntheticTokens<T> = synthetic_tokens::Module<T>;
type SynthesisResult<T> = result::Result<<T as synthetic_tokens::Trait>::Balance, Error>;

impl<T: Trait> Module<T> {
	fn _mint(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		collateral: T::Balance,
		max_slippage: Permill,
	) -> SynthesisResult<T> {
		ensure!(T::CollateralCurrency::balance(who) >= collateral, Error::BalanceTooLow);

		let price =
			T::PriceProvider::get_price(T::GetCollateralCurrencyId::get(), currency_id).ok_or(Error::NoPrice)?;
		let ask_price = Self::_get_ask_price(pool_id, currency_id, price, max_slippage)?;

		// synthetic = collateral / ask_price
		let synthetic_by_price = T::BalanceToPrice::convert(collateral)
			.checked_div(&ask_price)
			.ok_or(Error::NumOverflow)?;
		let synthetic = T::PriceToBalance::convert(synthetic_by_price);

		// synthetic_value = synthetic * price
		// `synthetic_value` is how much `synthetic` values in collateral unit.
		let synthetic_value = {
			let in_price_type = synthetic_by_price.checked_mul(&price).ok_or(Error::NumOverflow)?;
			T::PriceToBalance::convert(in_price_type)
		};
		// additional_collateral = synthetic_value * (1 + ratio) - synthetic_amount
		let additional_collateral =
			Self::_calc_additional_collateral_amount(pool_id, currency_id, collateral, synthetic_value)?;

		ensure!(
			T::LiquidityPoolsCurrency::balance(pool_id) >= additional_collateral,
			Error::LiquidityProviderBalanceTooLow,
		);

		// mint synthetic
		T::MultiCurrency::deposit(currency_id, who, synthetic).map_err(|e| e.into())?;

		// collateralise
		T::CollateralCurrency::transfer(who, &<SyntheticTokens<T>>::account_id(), collateral)
			.expect("ensured enough balance of sender; qed");
		T::LiquidityPoolsCurrency::withdraw(&<SyntheticTokens<T>>::account_id(), pool_id, additional_collateral)
			.expect("ensured enough collateral in liquidity pool; qed");

		let total_collateral = collateral + additional_collateral;
		<SyntheticTokens<T>>::add_position(pool_id, currency_id, total_collateral, synthetic);

		Ok(synthetic)
	}

	fn _redeem(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		synthetic: T::Balance,
		max_slippage: Permill,
	) -> SynthesisResult<T> {
		ensure!(
			T::MultiCurrency::balance(currency_id, &who) >= synthetic,
			Error::BalanceTooLow
		);

		let price =
			T::PriceProvider::get_price(T::GetCollateralCurrencyId::get(), currency_id).ok_or(Error::NoPrice)?;
		// bid_price = price * (1 - bid_spread)
		let bid_price = Self::_get_bid_price(pool_id, currency_id, price, Some(max_slippage))?;

		// collateral = synthetic * bid_price
		let collateral = {
			let collateral_by_price = T::BalanceToPrice::convert(synthetic)
				.checked_mul(&bid_price)
				.ok_or(Error::NumOverflow)?;
			T::PriceToBalance::convert(collateral_by_price)
		};

		let (collateral_to_remove, refund_to_pool) =
			Self::_calc_remove_position(pool_id, currency_id, price, synthetic, collateral)?;

		ensure!(
			T::CollateralCurrency::balance(&<SyntheticTokens<T>>::account_id()) >= collateral + refund_to_pool,
			Error::NotEnoughLockedCollateralAvailable,
		);

		// TODO: calculate and add interest to `refund_to_pool`

		// burn synthetic
		T::MultiCurrency::withdraw(currency_id, who, synthetic).map_err(|e| e.into())?;

		// redeem collateral
		T::CollateralCurrency::transfer(&<SyntheticTokens<T>>::account_id(), who, collateral)
			.expect("ensured enough locked collateral; qed");
		T::LiquidityPoolsCurrency::deposit(&<SyntheticTokens<T>>::account_id(), pool_id, refund_to_pool)
			.expect("ensured enough locked collateral; qed");

		<SyntheticTokens<T>>::remove_position(pool_id, currency_id, collateral_to_remove, synthetic);

		Ok(collateral)
	}

	fn _liquidate(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		synthetic: T::Balance,
	) -> SynthesisResult<T> {
		ensure!(
			T::MultiCurrency::balance(currency_id, &who) >= synthetic,
			Error::BalanceTooLow
		);

		let price =
			T::PriceProvider::get_price(T::GetCollateralCurrencyId::get(), currency_id).ok_or(Error::NoPrice)?;
		let bid_price = Self::_get_bid_price(pool_id, currency_id, price, None)?;
		// collateral = synthetic * bid_price
		let collateral = {
			let in_price = T::BalanceToPrice::convert(synthetic)
				.checked_mul(&bid_price)
				.ok_or(Error::NumOverflow)?;
			T::PriceToBalance::convert(in_price)
		};

		let (collateral_to_remove, refund_to_pool, incentive) =
			Self::_calc_remove_position_and_incentive(pool_id, currency_id, price, synthetic, collateral)?;

		// TODO: calculate and add interest to `refund_to_pool`

		// burn synthetic
		T::MultiCurrency::withdraw(currency_id, who, synthetic).map_err(|e| e.into())?;

		// Give liquidator collateral and incentive.
		let collateral_with_incentive = collateral.checked_add(&incentive).ok_or(Error::NumOverflow)?;
		T::CollateralCurrency::transfer(&<SyntheticTokens<T>>::account_id(), who, collateral_with_incentive)
			.expect("ensured enough locked collateral; qed");

		// refund to pool
		T::LiquidityPoolsCurrency::deposit(&<SyntheticTokens<T>>::account_id(), pool_id, refund_to_pool)
			.expect("ensured enough locked collateral; qed");

		<SyntheticTokens<T>>::remove_position(pool_id, currency_id, collateral_to_remove, synthetic);

		Ok(collateral)
	}
}

// other private methods

impl<T: Trait> Module<T> {
	/// Get ask price from liquidity pool for a given currency. Would fail if price could not meet max slippage.
	///
	/// ask_price = price * (1 + ask_spread)
	fn _get_ask_price(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		price: Price,
		max_slippage: Permill,
	) -> result::Result<Price, Error> {
		let ask_spread = T::LiquidityPoolsConfig::get_ask_spread(pool_id, currency_id);

		if ask_spread.deconstruct() > max_slippage.deconstruct() {
			return Err(Error::SlippageTooHigh);
		}

		let spread_amount = price.checked_mul(&ask_spread.into()).expect("ask_spread < 1; qed");
		price.checked_add(&spread_amount).ok_or(Error::NumOverflow)
	}

	/// Get bid price from liquidity pool for a given currency. Would fail if price could not meet max slippage.
	///
	/// ask_price = price * (1 - bid_spread)
	fn _get_bid_price(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		price: Price,
		max_slippage: Option<Permill>,
	) -> result::Result<Price, Error> {
		let bid_spread = T::LiquidityPoolsConfig::get_bid_spread(pool_id, currency_id);

		if let Some(m) = max_slippage {
			if bid_spread.deconstruct() > m.deconstruct() {
				return Err(Error::SlippageTooHigh);
			}
		}

		let spread_amount = price.checked_mul(&bid_spread.into()).expect("bid_spread < 1; qed");
		Ok(price.checked_sub(&spread_amount).expect("price > spread_amount; qed"))
	}

	/// Calculate liquidity provider's collateral parts:
	/// 	synthetic_value * (1 + ratio) - collateral
	fn _calc_additional_collateral_amount(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		collateral: T::Balance,
		synthetic_value: T::Balance,
	) -> SynthesisResult<T> {
		let with_additional_collateral = Self::_with_additional_collateral(pool_id, currency_id, synthetic_value)?;

		// would not overflow as long as `ratio` bigger than `ask_spread`, not likely to happen in real world case,
		// but better to be safe than sorry
		with_additional_collateral
			.checked_sub(&collateral)
			.ok_or(Error::NegativeAdditionalCollateralAmount)
	}

	/// Returns `collateral * (1 + ratio)`.
	fn _with_additional_collateral(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		collateral: T::Balance,
	) -> SynthesisResult<T> {
		let ratio = T::LiquidityPoolsConfig::get_additional_collateral_ratio(pool_id, currency_id);
		// should never overflow as ratio <= 1
		let additional = collateral * T::PriceToBalance::convert(ratio.into());

		collateral.checked_add(&additional).ok_or(Error::NumOverflow)
	}

	/// Calculate position change for a remove, if ok, return with `(collateral_to_remove, refund_to_pool)`
	fn _calc_remove_position(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		price: Price,
		synthetic: T::Balance,
		collateral: T::Balance,
	) -> result::Result<(T::Balance, T::Balance), Error> {
		let (collateral_position, synthetic_position) = <SyntheticTokens<T>>::get_position(pool_id, currency_id);

		ensure!(
			synthetic_position >= synthetic,
			Error::LiquidityPoolSyntheticPositionTooLow,
		);
		let new_synthetic_position = synthetic_position
			.checked_sub(&synthetic)
			.expect("ensured high enough synthetic position; qed");

		// new_synthetic_value = new_synthetic_position * price
		let new_synthetic_value = {
			let in_price_type = T::BalanceToPrice::convert(new_synthetic_position)
				.checked_mul(&price)
				.ok_or(Error::NumOverflow)?;
			T::PriceToBalance::convert(in_price_type)
		};
		let required_collateral = Self::_with_additional_collateral(pool_id, currency_id, new_synthetic_value)?;

		let mut collateral_to_remove = collateral;
		let mut refund_to_pool = Zero::zero();
		// TODO: handle the case `required_collateral > collateral_position`
		if required_collateral <= collateral_position {
			collateral_to_remove = collateral_position
				.checked_sub(&required_collateral)
				.expect("ensured high enough collateral position; qed");
			// TODO: handle the case zero `refund_to_pool`
			refund_to_pool = collateral_to_remove
				.checked_sub(&collateral)
				.ok_or(Error::LiquidityPoolCollateralPositionTooLow)?;
		}

		Ok((collateral_to_remove, refund_to_pool))
	}

	/// Calculate position change and incentive for a remove.
	///
	/// If `Ok`, return with `(collateral_to_remove, refund_to_pool, incentive)`
	fn _calc_remove_position_and_incentive(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		price: Price,
		synthetic: T::Balance,
		collateral: T::Balance,
	) -> result::Result<(T::Balance, T::Balance, T::Balance), Error> {
		let (collateral_position, synthetic_position) = <SyntheticTokens<T>>::get_position(pool_id, currency_id);
		ensure!(
			synthetic_position >= synthetic,
			Error::LiquidityPoolSyntheticPositionTooLow,
		);
		ensure!(
			collateral_position >= collateral,
			Error::LiquidityPoolCollateralPositionTooLow,
		);

		// synthetic_position_value = synthetic_position * price
		let synthetic_position_value = {
			let in_price = T::BalanceToPrice::convert(synthetic_position)
				.checked_mul(&price)
				.ok_or(Error::NumOverflow)?;
			T::PriceToBalance::convert(in_price)
		};
		// if synthetic position not backed by enough collateral, no incentive
		if collateral_position <= synthetic_position_value {
			return Ok((collateral, Zero::zero(), Zero::zero()));
		}

		// current_ratio = collateral_position / synthetic_position_value
		let current_ratio = {
			let collateral_position_u128: u128 =
				TryInto::<u128>::try_into(collateral_position).map_err(|_| Error::BalanceToU128Failed)?;
			let synthetic_position_value_u128: u128 =
				TryInto::<u128>::try_into(synthetic_position_value).map_err(|_| Error::BalanceToU128Failed)?;
			FixedU128::from_rational(collateral_position_u128, synthetic_position_value_u128)
		};

		// in safe position if ratio >= liquidation_ratio
		let one = FixedU128::from_rational(1, 1);
		let liquidation_ratio = <SyntheticTokens<T>>::liquidation_ratio_or_default(currency_id);
		let safe_ratio_threshold = Into::<FixedU128>::into(liquidation_ratio).saturating_add(one);
		ensure!(current_ratio < safe_ratio_threshold, Error::StillInSafePosition);

		let new_synthetic_position = synthetic_position
			.checked_sub(&synthetic)
			.expect("ensured high enough synthetic position; qed");
		let new_collateral_position = collateral_position
			.checked_sub(&collateral)
			.expect("ensured high enough collateral position; qed");

		// with_current_ratio = new_synthetic_position * price * current_ratio
		let with_current_ratio = {
			let new_synthetic_position_value = T::BalanceToPrice::convert(new_synthetic_position)
				.checked_mul(&price)
				.ok_or(Error::NumOverflow)?;
			let with_current_ratio = new_synthetic_position_value
				.checked_mul(&current_ratio)
				.ok_or(Error::NumOverflow)?;
			T::PriceToBalance::convert(with_current_ratio)
		};

		if new_collateral_position > with_current_ratio {
			// available_for_incentive = new_collateral_position - with_current_ratio
			let available_for_incentive = new_collateral_position
				.checked_sub(&with_current_ratio)
				.expect("ensured new collateral position higher; qed");
			let incentive_ratio = <SyntheticTokens<T>>::incentive_ratio(currency_id, current_ratio);
			// incentive = available_for_incentive * incentive_ratio
			let incentive = available_for_incentive
				.checked_mul(&T::PriceToBalance::convert(incentive_ratio))
				.ok_or(Error::NumOverflow)?;

			let refund_to_pool = available_for_incentive
				.checked_sub(&incentive)
				.expect("available_for_incentive > incentive; qed");
			let collateral_with_incentive = collateral.checked_add(&incentive).ok_or(Error::NumOverflow)?;
			let collateral_with_incentive_and_refund = collateral_with_incentive
				.checked_add(&refund_to_pool)
				.ok_or(Error::NumOverflow)?;
			Ok((collateral_with_incentive_and_refund, refund_to_pool, incentive))
		} else {
			// no more incentive could be given
			Ok((collateral, Zero::zero(), Zero::zero()))
		}
	}
}
