#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use rstd::result;
use sr_primitives::{
	traits::{CheckedAdd, CheckedSub, Convert, Zero},
	Permill,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_signed};

use orml_prices::Price;
use orml_traits::{BasicCurrency, MultiCurrency, PriceProvider};

use traits::{LiquidityPoolsConfig, LiquidityPoolsCurrency};

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
		/// (who, synthetic_token_id, liquidity_pool_id, collateral_amount, minted_amount)
		Minted(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),
		/// Synthetic token redeemed.
		/// (who, synthetic_token_id, liquidity_pool_id, collateral_amount, redeemed_amount)
		Redeemed(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),
		/// Synthetic token liquidated.
		/// (who, synthetic_token_id, liquidity_pool_id, collateral_amount, synthetic_token_amount)
		Liquidated(AccountId, CurrencyId, LiquidityPoolId, Balance, Balance),
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn mint(origin,
			pool_id: T::LiquidityPoolId,
			currency_id: T::CurrencyId,
			collateral_amount: T::Balance,
			max_slippage: Permill
		) {
			let who = ensure_signed(origin)?;
			let minted = Self::_mint(&who, pool_id, currency_id, collateral_amount, max_slippage)?;
			Self::deposit_event(RawEvent::Minted(who, currency_id, pool_id, collateral_amount, minted));
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
		LiquidityPoolSyntheticPositionTooLow,
		NegativeAdditionalCollateralAmount,
		NotEnoughCollateralInLiquidityPool,
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

		T::MultiCurrency::deposit(currency_id, who, synthetic).map_err(|e| e.into())?;

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
		let bid_price = Self::_get_bid_price(pool_id, currency_id, price, max_slippage)?;
		// collateral = synthetic * bid_price
		let collateral_by_price = T::BalanceToPrice::convert(synthetic)
			.checked_mul(&bid_price)
			.ok_or(Error::NumOverflow)?;
		let collateral = T::PriceToBalance::convert(collateral_by_price);

		let (collateral_to_remove, refund_to_pool) =
			Self::_calc_remove_position(currency_id, pool_id, price, synthetic, collateral)?;

		// TODO: add interest to `refund_to_pool`

		Ok(collateral)
	}

	fn _liquidate(
		_who: &T::AccountId,
		_pool_id: T::LiquidityPoolId,
		_currency_id: T::CurrencyId,
		_synthetic_token_amount: T::Balance,
	) -> SynthesisResult<T> {
		unimplemented!()
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
		max_slippage: Permill,
	) -> result::Result<Price, Error> {
		let bid_spread = T::LiquidityPoolsConfig::get_bid_spread(pool_id, currency_id);

		if bid_spread.deconstruct() > max_slippage.deconstruct() {
			return Err(Error::SlippageTooHigh);
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
			Error::LiquidityPoolSyntheticPositionTooLow
		);
		let new_synthetic_position = synthetic_position
			.checked_sub(&synthetic)
			.expect("ensured enough synthetic in liquidity pool; qed");

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
				.ok_or(Error::NotEnoughCollateralInLiquidityPool)?;
		}

		Ok((collateral_to_remove, refund_to_pool))
	}
}
