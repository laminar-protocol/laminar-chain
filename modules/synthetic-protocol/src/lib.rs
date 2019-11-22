#![cfg_attr(not(feature = "std"), no_std)]

use palette_support::{decl_error, decl_event, decl_module, decl_storage, Parameter};
use sr_primitives::{
	traits::{MaybeSerializeDeserialize, Member, SimpleArithmetic},
	Permill,
};
// FIXME: `pallet/palette-` prefix should be used for all pallet modules, but currently `palette_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use palette_system as system;

use orml_traits::PriceProvider;

pub trait Trait: palette_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as palette_system::Trait>::Event>;
	type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize;
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type Price: From<Self::Balance> + Into<Self::Balance>;
	type PriceProvider: PriceProvider<Self::CurrencyId, Self::Price>;
	type LiquidityPoolId: Parameter + Member + Copy + MaybeSerializeDeserialize;
}

const MAX_SPREAD: Permill = Permill::from_percent(3); // TODO: set this

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticProtocol {}
}

decl_event! {
	pub enum Event<T> where
		<T as palette_system::Trait>::AccountId,
		CurrencyId = <T as Trait>::CurrencyId,
		Balance = <T as Trait>::Balance,
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
	pub enum Error {}
}

impl<T: Trait> Module<T> {
	fn _mint(
		who: T::AccountId,
		currency_id: T::CurrencyId,
		pool_id: T::LiquidityPoolId,
		collateral_amount: T::Balance,
		max_slippage: Permill,
	) {
		unimplemented!()
	}

	fn _redeem(
		who: T::AccountId,
		currency_id: T::CurrencyId,
		pool_id: T::LiquidityPoolId,
		synthetic_token_amount: T::Balance,
		max_slippage: Permill,
	) {
		unimplemented!()
	}

	fn _liquidate(
		who: T::AccountId,
		currency_id: T::CurrencyId,
		pool_id: T::LiquidityPoolId,
		synthetic_token_amount: T::Balance,
	) {
		unimplemented!()
	}
}
