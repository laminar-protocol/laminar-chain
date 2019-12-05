#![cfg_attr(not(feature = "std"), no_std)]

mod liquidity_pool_option;
mod mock;
mod tests;

pub use liquidity_pool_option::LiquidityPoolOption;

use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch::Result, ensure, Parameter};
use frame_system::{self as system, ensure_signed};
use rstd::result;
use sp_runtime::{
	traits::{CheckedAdd, MaybeSerializeDeserialize, Member, One, SimpleArithmetic, Zero},
	Perbill,
};

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type LiquidityPoolId: Parameter + Member + Copy + Ord + Default + SimpleArithmetic;
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type CurrencyId: Parameter + Member + Copy + Ord + Default;
}

decl_storage! {
	trait Store for Module<T: Trait> as LiquidityPools {
		pub NextPoolId get(fn next_pool_id) build(|_| T::LiquidityPoolId::zero()): T::LiquidityPoolId;
		pub Owners get(fn owners): map T::LiquidityPoolId => Option<T::AccountId>;
		pub LiquidityPoolOptions get(fn liquidity_pool_options): double_map T::LiquidityPoolId, blake2_256(T::CurrencyId) => Option<LiquidityPoolOption>;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::LiquidityPoolId,
	{
		LiquidityPoolOptionCreated(AccountId, LiquidityPoolId),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn create_pool(origin, currency_id: T::CurrencyId) -> Result {
			let who = ensure_signed(origin)?;
			Self::_create_pool(who, currency_id).map_err(|e| e.into())
		}

		pub fn disable_pool(origin, pool_id: T::LiquidityPoolId) -> Result {
			let who = ensure_signed(origin)?;
			Self::_disable_pool(who, pool_id).map_err(|e| e.into())
		}

		pub fn remove_pool(origin, pool_id: T::LiquidityPoolId) -> Result {
			let who = ensure_signed(origin)?;
			Self::_remove_pool(who, pool_id).map_err(|e| e.into())
		}

		pub fn deposit_liquidity(origin, pool_id: T::LiquidityPoolId, amount: T::Balance) -> Result {
			// TODO: Add money to this pool
			Ok(())
		}

		pub fn set_spread(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, ask: Perbill, bid: Perbill) -> Result {
			let who = ensure_signed(origin)?;
			Self::_set_spread(who, pool_id, currency_id, ask, bid).map_err(|e| e.into())
		}
	}
}

decl_error! {
	// LiquidityPools module errors
	pub enum Error {
		NoPermission,
		CannotCreateMorePool,
		PoolNotFound,
	}
}

impl<T: Trait> Module<T> {
	pub fn is_owner(pool_id: T::LiquidityPoolId, who: T::AccountId) -> bool {
		Self::owners(pool_id) == Some(who)
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _create_pool(who: T::AccountId, currency_id: T::CurrencyId) -> result::Result<(), Error> {
		let pool_option = LiquidityPoolOption::default();

		let pool_id = Self::next_pool_id();
		<Owners<T>>::insert(pool_id, &who);
		<LiquidityPoolOptions<T>>::insert(pool_id, currency_id, pool_option);

		let next_pool_id = match pool_id.checked_add(&One::one()) {
			Some(id) => id,
			None => return Err(Error::CannotCreateMorePool),
		};
		<NextPoolId<T>>::put(next_pool_id);

		Self::deposit_event(RawEvent::LiquidityPoolOptionCreated(who, pool_id));
		Ok(())
	}

	fn _disable_pool(who: T::AccountId, pool_id: T::LiquidityPoolId) -> result::Result<(), Error> {
		ensure!(Self::owners(pool_id) == Some(who), Error::NoPermission);
		// TODO: Disable all tokens for this pool
		Ok(())
	}

	fn _remove_pool(who: T::AccountId, pool_id: T::LiquidityPoolId) -> result::Result<(), Error> {
		ensure!(Self::owners(pool_id) == Some(who), Error::NoPermission);
		// TODO: No outstanding positions
		// TODO: Withdraw all liquidity and remove this pool
		Ok(())
	}

	fn _set_spread(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		ask: Perbill,
		bid: Perbill,
	) -> result::Result<(), Error> {
		ensure!(Self::owners(pool_id) == Some(who), Error::NoPermission);

		match <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id) {
			Some(mut pool_option) => {
				pool_option.bid_spread = bid;
				pool_option.ask_spread = ask;
				<LiquidityPoolOptions<T>>::insert(pool_id, currency_id, pool_option);
				Ok(())
			}
			None => Err(Error::PoolNotFound),
		}
	}
}
