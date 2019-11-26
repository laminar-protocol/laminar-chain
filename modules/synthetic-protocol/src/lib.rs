#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter};
use rstd::result;
use sr_primitives::{
	traits::{MaybeSerializeDeserialize, Member},
	Permill,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system as system;

use orml_traits::{BasicCurrency, MultiCurrency, PriceProvider};

type BalanceOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::Balance;
type CurrencyIdOf<T> =
	<<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::CurrencyId;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId>;
	type GetBaseCurrencyId: Get<CurrencyIdOf<Self>>;
	type Price: From<BalanceOf<Self>> + Into<BalanceOf<Self>>;
	type PriceProvider: PriceProvider<CurrencyIdOf<Self>, Self::Price>;
	type LiquidityPoolId: Parameter + Member + Copy + MaybeSerializeDeserialize;
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
	}
}

type SynthesisResult<T> = result::Result<BalanceOf<T>, Error>;

impl<T: Trait> Module<T> {
	fn _mint(
		who: T::AccountId,
		currency_id: CurrencyIdOf<T>,
		pool_id: T::LiquidityPoolId,
		collateral_amount: BalanceOf<T>,
		max_slippage: Permill,
	) -> SynthesisResult<T> {
		//		ensure!(T::BaseCurrency::balance(who) >= collateral_amount, Error::BalanceTooLow);
		//		// TODO: Token white list? maybe not needed as we use enum as currency id.
		//
		//		let price = T::PriceProvider::get_price(T::GetBaseCurrencyId::get(), currency_id);
		//
		//		Ok(())
		unimplemented!()
	}

	fn _redeem(
		who: T::AccountId,
		currency_id: CurrencyIdOf<T>,
		pool_id: T::LiquidityPoolId,
		synthetic_token_amount: BalanceOf<T>,
		max_slippage: Permill,
	) -> SynthesisResult<T> {
		unimplemented!()
	}

	fn _liquidate(
		who: T::AccountId,
		currency_id: CurrencyIdOf<T>,
		pool_id: T::LiquidityPoolId,
		synthetic_token_amount: BalanceOf<T>,
	) -> SynthesisResult<T> {
		unimplemented!()
	}
}
