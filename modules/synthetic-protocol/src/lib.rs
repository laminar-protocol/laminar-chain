#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter};
use rstd::result;
use sr_primitives::{
	traits::{Convert, MaybeSerializeDeserialize, Member},
	Permill,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system as system;

use orml_traits::{BasicCurrency, MultiCurrency, PriceProvider};
use orml_prices::Price;

use traits::LiquidityPoolsConfig;

type BalanceOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::Balance;
type CurrencyIdOf<T> =
	<<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::CurrencyId;
type ErrorOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::Error;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId>;
	type BaseCurrency: BasicCurrency<Self::AccountId, Balance = BalanceOf<Self>, Error = ErrorOf<Self>>;
	type GetBaseCurrencyId: Get<CurrencyIdOf<Self>>;
	type PriceProvider: PriceProvider<CurrencyIdOf<Self>, Price>;
	type LiquidityPoolId: Parameter + Member + Copy + MaybeSerializeDeserialize;
	type LiquidityPoolsConfig: LiquidityPoolsConfig<
		CurrencyId = CurrencyIdOf<Self>,
		LiquidityPoolId = Self::LiquidityPoolId,
	>;
	type BalanceToPrice: Convert<BalanceOf<Self>, Price>;
	type PriceToBalance: Convert<Price, BalanceOf<Self>>;
}

const MAX_SPREAD: Permill = Permill::from_percent(3); // TODO: set this

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticProtocol {}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		CurrencyId = CurrencyIdOf<T>,
		Balance = BalanceOf<T>,
		LiquidityPoolId = <T as Trait>::LiquidityPoolId,
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
	}
}

decl_error! {
	pub enum Error
	{
		BalanceTooLow,
		SlippageTooHigh,
		NumOverflow,
		NoPrice,
	}
}

// Dispatch calls

type SynthesisResult<T> = result::Result<BalanceOf<T>, Error>;
impl<T: Trait> Module<T> {
	fn _mint(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: CurrencyIdOf<T>,
		collateral_amount: BalanceOf<T>,
		max_slippage: Permill,
	) -> SynthesisResult<T> {
		ensure!(T::BaseCurrency::balance(who) >= collateral_amount, Error::BalanceTooLow);
		// TODO: Token white list? maybe not needed as we use enum as currency id.

		let price = T::PriceProvider::get_price(T::GetBaseCurrencyId::get(), currency_id).ok_or(Error::NoPrice)?;
		let ask_price = Self::_get_ask_price(pool_id, currency_id, price, max_slippage)?;
		let minted_amount = ask_price
			.checked_mul(&T::BalanceToPrice::convert(collateral_amount))
			.ok_or(Error::NumOverflow)?;

		// TODO: additional collateral amount?

//		<SyntheticTokensOf<T>>::add_position(who, pool_id, currency_id, collateral_amount, minted_amount);

		// deposit minted tokens to `who`
		T::MultiCurrency::deposit(currency_id, who, T::PriceToBalance::convert(minted_amount))
			.map_err(|err| Error::Other(err.into()));
		// TODO: transfer collateral to?

		Ok(T::PriceToBalance::convert(minted_amount))
	}

	fn _redeem(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: CurrencyIdOf<T>,
		synthetic_token_amount: BalanceOf<T>,
		max_slippage: Permill,
	) -> SynthesisResult<T> {
		unimplemented!()
	}

	fn _liquidate(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: CurrencyIdOf<T>,
		synthetic_token_amount: BalanceOf<T>,
	) -> SynthesisResult<T> {
		unimplemented!()
	}
}

// Price

impl<T: Trait> Module<T> {
	/// Get ask price from liquidity pool for a given currency. Would fail if price could not meet max slippage.
	fn _get_ask_price(
		pool_id: T::LiquidityPoolId,
		currency_id: CurrencyIdOf<T>,
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
}
