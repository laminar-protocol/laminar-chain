#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	storage::IterableStorageMap,
	traits::{Currency, EnsureOrigin, Get, ReservableCurrency},
	weights::DispatchClass,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use orml_traits::BasicCurrency;
use primitives::{Balance, IdentityInfo, LiquidityPoolId};
use sp_runtime::{
	traits::{AccountIdConversion, One},
	DispatchResult, ModuleId,
};
use sp_std::{prelude::*, result};
use traits::{BaseLiquidityPoolManager, LiquidityPools, OnDisableLiquidityPool, OnRemoveLiquidityPool};

mod mock;
mod tests;

type DepositBalanceOf<T, I> =
	<<T as Trait<I>>::DepositCurrency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub trait Trait<I: Instance = DefaultInstance>: system::Trait {
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Balance>;
	type PoolManager: BaseLiquidityPoolManager<LiquidityPoolId, Balance>;
	type ExistentialDeposit: Get<Balance>;
	type Deposit: Get<DepositBalanceOf<Self, I>>;
	type DepositCurrency: ReservableCurrency<Self::AccountId>;
	type ModuleId: Get<ModuleId>;
	type OnDisableLiquidityPool: OnDisableLiquidityPool;
	type OnRemoveLiquidityPool: OnRemoveLiquidityPool;
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as BaseLiquidityPools {
		pub NextPoolId get(fn next_pool_id): LiquidityPoolId;
		pub Owners get(fn owners): map hasher(twox_64_concat) LiquidityPoolId => Option<(T::AccountId, LiquidityPoolId)>;
		pub Balances get(fn balances): map hasher(twox_64_concat) LiquidityPoolId => Balance;
		/// Store identity info of liquidity pool LiquidityPoolId => Option<(IdentityInfo, VerifyStatus)>
		pub IdentityInfos get(fn identity_infos): map hasher(twox_64_concat) LiquidityPoolId => Option<(IdentityInfo, bool)>;
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
		/// Set identity (who, pool_id)
		SetIdentity(AccountId, LiquidityPoolId),
		/// Verify identity (pool_id)
		VerifyIdentity(LiquidityPoolId),
		/// Clear identity (who, pool_id)
		ClearIdentity(AccountId, LiquidityPoolId),
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
		IdentityInfoTooLong,
		IdentityNotFound,
	}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {
		type Error = Error<T, I>;

		fn deposit_event() = default;

		const ExistentialDeposit: Balance = T::ExistentialDeposit::get();
		const Deposit: DepositBalanceOf<T,I> = T::Deposit::get();

		#[weight = 10_000]
		pub fn create_pool(origin) {
			let who = ensure_signed(origin)?;
			let pool_id = Self::_create_pool(&who)?;
			Self::deposit_event(RawEvent::LiquidityPoolCreated(who, pool_id));
		}

		#[weight = 10_000]
		pub fn disable_pool(origin, #[compact] pool_id: LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_disable_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolDisabled(who, pool_id));
		}

		#[weight = 50_000]
		pub fn remove_pool(origin, #[compact] pool_id: LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_remove_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolRemoved(who, pool_id));
		}

		#[weight = (10_000, DispatchClass::Operational)]
		pub fn deposit_liquidity(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_deposit_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::DepositLiquidity(who, pool_id, amount));
		}

		#[weight = 10_000]
		pub fn withdraw_liquidity(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
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

		#[weight = 10_000]
		pub fn set_identity(origin, #[compact] pool_id: LiquidityPoolId, identity_info: IdentityInfo) {
			let who = ensure_signed(origin)?;
			Self::_set_identity(&who, pool_id, identity_info)?;
			Self::deposit_event(RawEvent::SetIdentity(who, pool_id));
		}

		#[weight = 10_000]
		pub fn verify_identity(origin, #[compact] pool_id: LiquidityPoolId) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;

			Self::_verify_identity(pool_id)?;
			Self::deposit_event(RawEvent::VerifyIdentity(pool_id));
		}

		#[weight = 10_000]
		pub fn clear_identity(origin, #[compact] pool_id: LiquidityPoolId) {
			let who = ensure_signed(origin)?;

			ensure!(
				<IdentityInfos<I>>::contains_key(&pool_id),
				Error::<T, I>::IdentityNotFound
			);

			Self::_clear_identity(&who, pool_id)?;
			Self::deposit_event(RawEvent::ClearIdentity(who, pool_id));
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

	/// Check if pool exists
	fn pool_exists(pool_id: LiquidityPoolId) -> bool {
		<Owners<T, I>>::contains_key(&pool_id)
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

		// clear_identity
		Self::_clear_identity(who, pool_id)?;

		let balance = Self::balances(&pool_id);
		// transfer balance to pool owner
		T::LiquidityCurrency::transfer(&Self::account_id(), who, balance)?;

		<Balances<I>>::remove(&pool_id);
		<Owners<T, I>>::remove(&pool_id);

		T::OnRemoveLiquidityPool::on_remove(pool_id);

		Ok(())
	}

	fn _deposit_liquidity(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(Self::pool_exists(pool_id), Error::<T, I>::PoolNotFound);

		let balance = Self::balances(&pool_id);
		let new_balance = balance.checked_add(amount).ok_or(Error::<T, I>::CannotDepositAmount)?;

		// transfer amount to this pool
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount)?;
		// update balance
		<Balances<I>>::insert(&pool_id, new_balance);

		Ok(())
	}

	fn _withdraw_liquidity(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(Self::pool_exists(pool_id), Error::<T, I>::PoolNotFound);

		let new_balance = Self::balances(&pool_id)
			.checked_sub(amount)
			.ok_or(Error::<T, I>::CannotWithdrawAmount)?;

		// transfer amount to account
		T::LiquidityCurrency::transfer(&Self::account_id(), who, amount)?;

		// update balance
		<Balances<I>>::insert(&pool_id, new_balance);

		Ok(())
	}

	fn _set_identity(who: &T::AccountId, pool_id: LiquidityPoolId, identity_info: IdentityInfo) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, &who), Error::<T, I>::NoPermission);
		ensure!(
			identity_info.legal.len() <= 100
				&& identity_info.display.len() <= 200
				&& identity_info.web.len() <= 100
				&& identity_info.email.len() <= 50
				&& identity_info.image_url.len() <= 100,
			Error::<T, I>::IdentityInfoTooLong
		);

		if <IdentityInfos<I>>::contains_key(&pool_id) {
			<IdentityInfos<I>>::insert(&pool_id, (identity_info, false));
		} else {
			// reserve deposit from owner
			T::DepositCurrency::reserve(who, T::Deposit::get())?;

			<IdentityInfos<I>>::insert(&pool_id, (identity_info, false));
		}

		Ok(())
	}

	fn _verify_identity(pool_id: LiquidityPoolId) -> DispatchResult {
		let (identity_info, _) = Self::identity_infos(pool_id).ok_or(Error::<T, I>::IdentityNotFound)?;
		<IdentityInfos<I>>::insert(&pool_id, (identity_info, true));

		Ok(())
	}

	fn _clear_identity(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, &who), Error::<T, I>::NoPermission);

		if <IdentityInfos<I>>::contains_key(&pool_id) {
			T::DepositCurrency::unreserve(who, T::Deposit::get());
			<IdentityInfos<I>>::remove(&pool_id);
		}

		Ok(())
	}
}
