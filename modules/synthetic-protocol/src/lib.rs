#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use rstd::result;
use sr_primitives::{
	traits::{CheckedAdd, Convert},
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

		// minted = collateral / ask_price
		let minted_by_price = T::BalanceToPrice::convert(collateral)
			.checked_div(&ask_price)
			.ok_or(Error::NumOverflow)?;
		let minted = T::PriceToBalance::convert(minted_by_price);

		// minted_current_value = minted * price
		let minted_current_value = minted_by_price.checked_mul(&price).ok_or(Error::NumOverflow)?;
		// additional_collateral = minted_current_value * (1 + ratio) - minted_amount
		let additional_collateral = Self::_calc_additional_collateral_amount(
			pool_id,
			currency_id,
			minted,
			T::PriceToBalance::convert(minted_current_value),
		)?;

		ensure!(
			T::LiquidityPoolsCurrency::balance(pool_id) >= additional_collateral,
			Error::LiquidityProviderBalanceTooLow,
		);

		T::MultiCurrency::deposit(currency_id, who, minted).map_err(|e| e.into())?;

		T::CollateralCurrency::transfer(who, &<SyntheticTokens<T>>::account_id(), collateral)
			.expect("`who`'s balance checked above; qed");
		T::LiquidityPoolsCurrency::withdraw(&<SyntheticTokens<T>>::account_id(), pool_id, additional_collateral)
			.expect("liquidity pool balance checked above; qed");

		<SyntheticTokens<T>>::add_position(pool_id, currency_id, collateral, minted);

		Ok(minted)
	}

	fn _redeem(
		_who: &T::AccountId,
		_pool_id: T::LiquidityPoolId,
		_currency_id: T::CurrencyId,
		_synthetic_token_amount: T::Balance,
		_max_slippage: Permill,
	) -> SynthesisResult<T> {
		unimplemented!()
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

		let spread_amount = price.checked_mul(&ask_spread.into()).ok_or(Error::NumOverflow)?;
		price.checked_add(&spread_amount).ok_or(Error::NumOverflow)
	}

	/// Calculate liquidity provider's collateral parts:
	/// 	minted_current_value * (1 + ratio) - minted_amount
	fn _calc_additional_collateral_amount(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		minted: T::Balance,
		minted_current_value: T::Balance,
	) -> SynthesisResult<T> {
		let ratio = T::LiquidityPoolsConfig::get_additional_collateral_ratio(pool_id, currency_id);
		// should never overflow as ratio <= 1
		let additional_value = minted_current_value * T::PriceToBalance::convert(ratio.into());
		let with_additional_value = minted_current_value
			.checked_add(&additional_value)
			.ok_or(Error::NumOverflow)?;
		// should never overflow
		Ok(with_additional_value - minted)
	}
}
