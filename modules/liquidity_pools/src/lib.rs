#![cfg_attr(not(feature = "std"), no_std)]

mod liquidity_pool;
mod mock;
mod tests;

pub use liquidity_pool::LiquidityPool;

use core::ops::Add;
use rstd::result;
use sr_primitives::traits::{Member, One, SimpleArithmetic};
use support::{decl_error, decl_event, decl_module, decl_storage, dispatch::Result, Parameter};
use system::ensure_signed;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type LiquidityPoolId: Parameter + Member + Copy + Ord + Default + SimpleArithmetic;
}

decl_storage! {
	trait Store for Module<T: Trait> as LiquidityPools {
		pub NextPoolId get(next_pool_id): T::LiquidityPoolId = T::LiquidityPoolId::one();
		pub Owners get(owners): map T::LiquidityPoolId => T::AccountId;
		pub LiquidityPools get(liquidity_pools): map T::LiquidityPoolId => LiquidityPool;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::LiquidityPoolId,
	{
		Dummy(AccountId),
		LiquidityPoolCreated(AccountId, LiquidityPoolId),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn create_pool(origin) -> Result {
			let who = ensure_signed(origin)?;
			Self::_create_pool(who).map_err(|e| e.into())
		}
	}
}

decl_error! {
	// LiquidityPools module errors
	pub enum Error {}
}

impl<T: Trait> Module<T> {
	pub fn is_owner(pool: T::LiquidityPoolId, who: T::AccountId) -> bool {
		<Owners<T>>::get(pool) == who
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _create_pool(who: T::AccountId) -> result::Result<(), Error> {
		let liquidity_pool = LiquidityPool::default();

		let pool_id = <NextPoolId<T>>::get();

		<Owners<T>>::insert(pool_id, &who);
		<LiquidityPools<T>>::insert(pool_id, liquidity_pool);

		let next_pool_id = T::LiquidityPoolId::one().add(pool_id);
		<NextPoolId<T>>::put(next_pool_id);

		Self::deposit_event(RawEvent::LiquidityPoolCreated(who, pool_id));
		Ok(())
	}
}
