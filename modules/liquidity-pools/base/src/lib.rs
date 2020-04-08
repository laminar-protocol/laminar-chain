#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageMap, traits::Get,
};
use frame_system::{self as system, ensure_signed};
use orml_traits::BasicCurrency;
use primitives::{Balance, LiquidityPoolId};
use sp_runtime::{
	traits::{AccountIdConversion, One},
	DispatchResult, ModuleId,
};
use sp_std::{prelude::*, result};
use traits::{LiquidityPoolManager, LiquidityPools, OnDisableLiquidityPool, OnRemoveLiquidityPool};

mod mock;
mod tests;

pub trait Trait<I: Instance = DefaultInstance>: system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Balance>;
	type PoolManager: LiquidityPoolManager<LiquidityPoolId, Balance>;
	type ExistentialDeposit: Get<Balance>;
	type ModuleId: Get<ModuleId>;
	type OnDisableLiquidityPool: OnDisableLiquidityPool;
	type OnRemoveLiquidityPool: OnRemoveLiquidityPool;
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as BaseLiquidityPools {
		pub NextPoolId get(fn next_pool_id): LiquidityPoolId;
		pub Owners get(fn owners): map hasher(twox_64_concat) LiquidityPoolId => Option<(T::AccountId, LiquidityPoolId)>;
		pub Balances get(fn balances): map hasher(twox_64_concat) LiquidityPoolId => Balance;
	}
}

decl_event!(
	pub enum Event<T, I=DefaultInstance> where
		<T as system::Trait>::AccountId,
	{
		/// Liquidity pool created (who, pool_id)
		LiquidityPoolCreated(AccountId, LiquidityPoolId),
		/// Liquidity pool disabled (who, pool_id)
		LiquidityPoolDisabled(AccountId, LiquidityPoolId),
		/// Liquidity pool removed (who, pool_id)
		LiquidityPoolRemoved(AccountId, LiquidityPoolId),
		/// Deposit liquidity (who, pool_id, amount)
		DepositLiquidity(AccountId, LiquidityPoolId, Balance),
		/// Withdraw liquidity (who, pool_id, amount)
		WithdrawLiquidity(AccountId, LiquidityPoolId, Balance),
	}
);

decl_error! {
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		NoPermission,
		CannotCreateMorePool,
		CannotRemovePool,
		CannotDepositAmount,
		CannotWithdrawAmount,
		CannotWithdrawExistentialDeposit,
		PoolNotFound,
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {
		type Error = Error<T, I>;

		fn deposit_event() = default;

		const ExistentialDeposit: Balance = T::ExistentialDeposit::get();

		pub fn create_pool(origin) {
			let who = ensure_signed(origin)?;
			let pool_id = Self::_create_pool(&who)?;
			Self::deposit_event(RawEvent::LiquidityPoolCreated(who, pool_id));
		}

		pub fn disable_pool(origin, pool_id: LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_disable_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolDisabled(who, pool_id));
		}

		pub fn remove_pool(origin, pool_id: LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_remove_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolRemoved(who, pool_id));
		}

		pub fn deposit_liquidity(origin, pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_deposit_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::DepositLiquidity(who, pool_id, amount));
		}

		pub fn withdraw_liquidity(origin, pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T, I>::NoPermission);

			T::PoolManager::ensure_can_withdraw(pool_id, amount)?;

			let new_balance = Self::balances(&pool_id).checked_sub(amount).ok_or(Error::<T, I>::CannotWithdrawAmount)?;

			// check minimum balance
			if new_balance < T::ExistentialDeposit::get() {
				return Err(Error::<T, I>::CannotWithdrawExistentialDeposit.into());
			}

			Self::_withdraw_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::WithdrawLiquidity(who, pool_id, amount));
		}
	}
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	pub fn is_owner(pool_id: LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::owners(pool_id).map_or(false, |(id, _)| &id == who)
	}
}

impl<T: Trait<I>, I: Instance> LiquidityPools<T::AccountId> for Module<T, I> {
	fn all() -> Vec<LiquidityPoolId> {
		<Owners<T, I>>::iter().map(|(_, (_, pool_id))| pool_id).collect()
	}

	fn is_owner(pool_id: LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::is_owner(pool_id, who)
	}

	/// Check collateral balance of `pool_id`.
	fn liquidity(pool_id: LiquidityPoolId) -> Balance {
		Self::balances(&pool_id)
	}

	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(source: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		Self::_deposit_liquidity(source, pool_id, amount)
	}

	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(dest: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		Self::_withdraw_liquidity(dest, pool_id, amount)
	}
}

// Private methods
impl<T: Trait<I>, I: Instance> Module<T, I> {
	fn _create_pool(who: &T::AccountId) -> result::Result<LiquidityPoolId, Error<T, I>> {
		let pool_id = Self::next_pool_id();
		// increment next pool id
		let next_pool_id = pool_id
			.checked_add(One::one())
			.ok_or(Error::<T, I>::CannotCreateMorePool)?;
		<NextPoolId<I>>::put(next_pool_id);
		// owner reference
		<Owners<T, I>>::insert(&pool_id, (who, pool_id));
		Ok(pool_id)
	}

	fn _disable_pool(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T, I>::NoPermission);

		T::OnDisableLiquidityPool::on_disable(pool_id);

		Ok(())
	}

	fn _remove_pool(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T, I>::NoPermission);
		ensure!(T::PoolManager::can_remove(pool_id), Error::<T, I>::CannotRemovePool);

		let balance = Self::balances(&pool_id);
		// transfer balance to pool owner
		T::LiquidityCurrency::transfer(&Self::account_id(), who, balance)?;

		<Balances<I>>::remove(&pool_id);
		<Owners<T, I>>::remove(&pool_id);

		T::OnRemoveLiquidityPool::on_remove(pool_id);

		Ok(())
	}

	fn _deposit_liquidity(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(<Owners<T, I>>::contains_key(&pool_id), Error::<T, I>::PoolNotFound);

		let balance = Self::balances(&pool_id);
		let new_balance = balance.checked_add(amount).ok_or(Error::<T, I>::CannotDepositAmount)?;

		// transfer amount to this pool
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount)?;
		// update balance
		<Balances<I>>::insert(&pool_id, new_balance);

		Ok(())
	}

	fn _withdraw_liquidity(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(<Owners<T, I>>::contains_key(&pool_id), Error::<T, I>::PoolNotFound);

		let new_balance = Self::balances(&pool_id)
			.checked_sub(amount)
			.ok_or(Error::<T, I>::CannotWithdrawAmount)?;

		// transfer amount to account
		T::LiquidityCurrency::transfer(&Self::account_id(), who, amount)?;

		// update balance
		<Balances<I>>::insert(&pool_id, new_balance);

		Ok(())
	}
}
