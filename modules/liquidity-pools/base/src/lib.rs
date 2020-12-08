#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	storage::IterableStorageMap,
	traits::{Currency, EnsureOrigin, Get, ReservableCurrency},
	weights::{DispatchClass, Weight},
};
use frame_system::ensure_signed;
use orml_traits::BasicCurrency;
use orml_utilities::with_transaction_result;
use primitives::{Balance, IdentityInfo, LiquidityPoolId};
use sp_runtime::{
	traits::{AccountIdConversion, One},
	DispatchResult, ModuleId, RuntimeDebug,
};
use sp_std::{prelude::*, result};
use traits::{BaseLiquidityPoolManager, LiquidityPools, OnDisableLiquidityPool, OnRemoveLiquidityPool};

mod default_weight;
mod mock;
mod tests;

pub trait WeightInfo {
	fn create_pool() -> Weight;
	fn disable_pool() -> Weight;
	fn remove_pool() -> Weight;
	fn deposit_liquidity() -> Weight;
	fn withdraw_liquidity() -> Weight;
	fn set_identity() -> Weight;
	fn verify_identity() -> Weight;
	fn clear_identity() -> Weight;
	fn transfer_liquidity_pool() -> Weight;
}

type IdentityDepositBalanceOf<T, I> =
	<<T as Config<I>>::IdentityDepositCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait Config<I: Instance = DefaultInstance>: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event<Self, I>> + Into<<Self as frame_system::Config>::Event>;

	/// The currency used for pool liquidity.
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Balance>;

	/// Manager of liquidity pools.
	type PoolManager: BaseLiquidityPoolManager<LiquidityPoolId, Balance>;

	/// Existential deposit of a liquidity pool.
	///
	/// Existential deposit cannot be withdrew.
	type ExistentialDeposit: Get<Balance>;

	/// The deposit amount needed for identity verification.
	type IdentityDeposit: Get<IdentityDepositBalanceOf<Self, I>>;

	/// The reservable currency for identity verification deposit.
	type IdentityDepositCurrency: ReservableCurrency<Self::AccountId>;

	/// Module Id of base liquidity pools module instance.
	type ModuleId: Get<ModuleId>;

	/// The receiver of the signal for when a liquidity pool is disabled.
	type OnDisableLiquidityPool: OnDisableLiquidityPool;

	/// The receiver of the signal for when a liquidity pool is removed.
	type OnRemoveLiquidityPool: OnRemoveLiquidityPool;

	/// Required origin for updating protocol options.
	type UpdateOrigin: EnsureOrigin<Self::Origin>;

	/// Weight information for the extrinsics in this module.
	type WeightInfo: WeightInfo;
}

/// Liquidity pool information.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct Pool<AccountId> {
	/// The owner of the liquidity pool.
	pub owner: AccountId,
	/// The balance of the liquidity pool.
	pub balance: Balance,
}
impl<AccountId> Pool<AccountId> {
	fn new(owner: AccountId, balance: Balance) -> Self {
		Pool { owner, balance }
	}
}

decl_storage! {
	trait Store for Module<T: Config<I>, I: Instance=DefaultInstance> as BaseLiquidityPools {
		/// Next available liquidity pool ID.
		pub NextPoolId get(fn next_pool_id): LiquidityPoolId;

		/// Liquidity pool information.
		///
		/// Returns `None` if no such pool exists.
		pub Pools get(fn pools): map hasher(twox_64_concat) LiquidityPoolId => Option<Pool<T::AccountId>>;

		/// Identity info of liquidity pools: `(identity_info, deposit_amount, is_verified)`.
		///
		/// Returns `None` if identity info of the pool not set or removed.
		pub IdentityInfos get(fn identity_infos): map hasher(twox_64_concat) LiquidityPoolId => Option<(IdentityInfo, IdentityDepositBalanceOf<T, I>, bool)>;
	}
}

decl_event!(
	pub enum Event<T, I=DefaultInstance> where
		<T as frame_system::Config>::AccountId,
	{
		/// Liquidity pool created: \[who, pool_id\]
		LiquidityPoolCreated(AccountId, LiquidityPoolId),

		/// Liquidity pool disabled: \[who, pool_id\]
		LiquidityPoolDisabled(AccountId, LiquidityPoolId),

		/// Liquidity pool removed: \[who, pool_id\]
		LiquidityPoolRemoved(AccountId, LiquidityPoolId),

		/// Liquidity deposited: \[who, pool_id, amount\]
		LiquidityDeposited(AccountId, LiquidityPoolId, Balance),

		/// Liquidity withdrew: \[who, pool_id, amount\]
		LiquidityWithdrew(AccountId, LiquidityPoolId, Balance),

		/// Identity set: \[who, pool_id\]
		IdentitySet(AccountId, LiquidityPoolId),

		/// Identity verified: \[pool_id\]
		IdentityVerified(LiquidityPoolId),

		/// Identity cleared: \[who, pool_id\]
		IdentityCleared(AccountId, LiquidityPoolId),

		/// Liquidity pool transferred to another owner: \[from, pool_id, to\]
		LiquidityPoolTransferred(AccountId, LiquidityPoolId, AccountId),
	}
);

decl_error! {
	/// Errors for the base liquidity pools module.
	pub enum Error for Module<T: Config<I>, I: Instance> {
		/// Caller doesn't have permission.
		NoPermission,

		/// No available pool id.
		NoAvailablePoolId,

		/// Can not remove a pool.
		///
		/// There is still liability, such as opened positions.
		CannotRemovePool,

		/// Liquidity amount overflows maximum.
		///
		/// Only happened when the liquidity currency went wrong and liquidity amount overflows the integer type.
		LiquidityOverflow,

		/// Not enough balance to withdraw.
		NotEnoughBalance,

		/// Cannot withdraw the existential deposit amount.
		CannotWithdrawExistentialDeposit,

		/// Pool not found.
		PoolNotFound,

		/// One of the identity information is too long.
		IdentityInfoTooLong,

		/// Identify information not found.
		IdentityInfoNotFound,
	}
}

decl_module! {
	pub struct Module<T: Config<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {
		type Error = Error<T, I>;

		fn deposit_event() = default;

		const ExistentialDeposit: Balance = T::ExistentialDeposit::get();
		const Deposit: IdentityDepositBalanceOf<T,I> = T::IdentityDeposit::get();

		/// Create a liquidity pool.
		///
		/// Caller would be the owner of created pool.
		#[weight = T::WeightInfo::create_pool()]
		pub fn create_pool(origin) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				let pool_id = Self::do_create_pool(&who)?;
				Self::deposit_event(RawEvent::LiquidityPoolCreated(who, pool_id));
				Ok(())
			})?;
		}

		/// Disable a liquidity pool.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::disable_pool()]
		pub fn disable_pool(origin, #[compact] pool_id: LiquidityPoolId) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_disable_pool(&who, pool_id)?;
				Self::deposit_event(RawEvent::LiquidityPoolDisabled(who, pool_id));
				Ok(())
			})?;
		}

		/// Remove a liquidity pool.
		///
		/// May only be called from the pool owner. Pools may only be removed when there is no liability.
		#[weight = T::WeightInfo::remove_pool()]
		pub fn remove_pool(origin, #[compact] pool_id: LiquidityPoolId) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_remove_pool(&who, pool_id)?;
				Self::deposit_event(RawEvent::LiquidityPoolRemoved(who, pool_id));
				Ok(())
			})?;
		}

		/// Deposit liquidity to a pool.
		#[weight = (T::WeightInfo::deposit_liquidity(), DispatchClass::Operational)]
		pub fn deposit_liquidity(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_deposit_liquidity(&who, pool_id, amount)?;
				Self::deposit_event(RawEvent::LiquidityDeposited(who, pool_id, amount));
				Ok(())
			})?;
		}

		/// Withdraw liquidity from a pool.
		#[weight = T::WeightInfo::withdraw_liquidity()]
		pub fn withdraw_liquidity(origin, #[compact] pool_id: LiquidityPoolId, #[compact] amount: Balance) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				ensure!(Self::is_owner(pool_id, &who), Error::<T, I>::NoPermission);

				T::PoolManager::ensure_can_withdraw(pool_id, amount)?;

				let new_balance = Self::balance(pool_id).checked_sub(amount).ok_or(Error::<T, I>::NotEnoughBalance)?;

				// check minimum balance
				if new_balance < T::ExistentialDeposit::get() {
					return Err(Error::<T, I>::CannotWithdrawExistentialDeposit.into());
				}

				Self::do_withdraw_liquidity(&who, pool_id, amount)?;
				Self::deposit_event(RawEvent::LiquidityWithdrew(who, pool_id, amount));

				Ok(())
			})?;
		}

		/// Set identity of a liquidity pool.
		///
		/// May only be called from the pool owner. `IdentityDeposit` amount of balance would be reserved.
		#[weight = T::WeightInfo::set_identity()]
		pub fn set_identity(origin, #[compact] pool_id: LiquidityPoolId, identity_info: IdentityInfo) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_set_identity(&who, pool_id, identity_info)?;
				Self::deposit_event(RawEvent::IdentitySet(who, pool_id));
				Ok(())
			})?;
		}

		/// Mark the identity of a liquidity pool as verified.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::verify_identity()]
		pub fn verify_identity(origin, #[compact] pool_id: LiquidityPoolId) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				Self::do_verify_identity(pool_id)?;
				Self::deposit_event(RawEvent::IdentityVerified(pool_id));
				Ok(())
			})?;
		}

		/// Remove the identity info of a liquidity pool.
		///
		/// May only be called from the pool owner. The reserved balance would be released.
		#[weight = T::WeightInfo::clear_identity()]
		pub fn clear_identity(origin, #[compact] pool_id: LiquidityPoolId) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;

				ensure!(
					<IdentityInfos<T, I>>::contains_key(&pool_id),
					Error::<T, I>::IdentityInfoNotFound
				);

				Self::do_clear_identity(&who, pool_id)?;
				Self::deposit_event(RawEvent::IdentityCleared(who, pool_id));

				Ok(())
			})?;
		}

		/// Transfer the ownership of the liquidity pool to `to`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::transfer_liquidity_pool()]
		pub fn transfer_liquidity_pool(origin, #[compact] pool_id: LiquidityPoolId, to: T::AccountId) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_transfer_liquidity_pool(&who, pool_id, &to)?;
				Self::deposit_event(RawEvent::LiquidityPoolTransferred(who, pool_id, to));
				Ok(())
			})?;
		}
	}
}

impl<T: Config<I>, I: Instance> Module<T, I> {
	pub fn account_id() -> T::AccountId {
		T::ModuleId::get().into_account()
	}

	pub fn is_owner(pool_id: LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::owner(pool_id).map_or(false, |ref owner| owner == who)
	}
}

impl<T: Config<I>, I: Instance> LiquidityPools<T::AccountId> for Module<T, I> {
	fn all() -> Vec<LiquidityPoolId> {
		// TODO: optimize once `iter_first_key` is ready
		<Pools<T, I>>::iter().map(|(pool_id, _)| pool_id).collect()
	}

	fn is_owner(pool_id: LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::is_owner(pool_id, who)
	}

	/// Check if pool exists
	fn pool_exists(pool_id: LiquidityPoolId) -> bool {
		<Pools<T, I>>::contains_key(&pool_id)
	}

	/// Check collateral balance of `pool_id`.
	fn liquidity(pool_id: LiquidityPoolId) -> Balance {
		Self::balance(pool_id)
	}

	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(source: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		Self::do_deposit_liquidity(source, pool_id, amount)
	}

	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(dest: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		Self::do_withdraw_liquidity(dest, pool_id, amount)
	}
}

// Storage getters and setters
impl<T: Config<I>, I: Instance> Module<T, I> {
	/// Balance of a liquidity pool.
	pub fn balance(pool_id: LiquidityPoolId) -> Balance {
		Self::pools(&pool_id).map_or(Default::default(), |pool| pool.balance)
	}

	/// Owner of a liquidity pool. Returns `None` is pool not found.
	pub fn owner(pool_id: LiquidityPoolId) -> Option<T::AccountId> {
		Self::pools(&pool_id).map(|pool| pool.owner)
	}

	fn set_balance(pool_id: LiquidityPoolId, balance: Balance) {
		if let Some(mut pool) = Self::pools(pool_id) {
			pool.balance = balance;
			<Pools<T, I>>::insert(&pool_id, pool);
		}
	}
}

// Dispatchable calls implementation
impl<T: Config<I>, I: Instance> Module<T, I> {
	fn do_create_pool(who: &T::AccountId) -> result::Result<LiquidityPoolId, Error<T, I>> {
		let pool_id = Self::next_pool_id();
		// increment next pool id
		let next_pool_id = pool_id
			.checked_add(One::one())
			.ok_or(Error::<T, I>::NoAvailablePoolId)?;
		<NextPoolId<I>>::put(next_pool_id);
		// owner reference
		<Pools<T, I>>::insert(&pool_id, Pool::new(who.clone(), Default::default()));
		Ok(pool_id)
	}

	fn do_disable_pool(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T, I>::NoPermission);

		T::OnDisableLiquidityPool::on_disable(pool_id);

		Ok(())
	}

	fn do_remove_pool(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T, I>::NoPermission);
		ensure!(T::PoolManager::can_remove(pool_id), Error::<T, I>::CannotRemovePool);

		// clear_identity
		Self::do_clear_identity(who, pool_id)?;

		let balance = Self::balance(pool_id);
		// transfer balance to pool owner
		T::LiquidityCurrency::transfer(&Self::account_id(), who, balance)?;

		<Pools<T, I>>::remove(&pool_id);

		T::OnRemoveLiquidityPool::on_remove(pool_id);

		Ok(())
	}

	fn do_deposit_liquidity(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(Self::pool_exists(pool_id), Error::<T, I>::PoolNotFound);

		let new_balance = Self::balance(pool_id)
			.checked_add(amount)
			.ok_or(Error::<T, I>::LiquidityOverflow)?;

		// transfer amount to this pool
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount)?;
		// update balance
		Self::set_balance(pool_id, new_balance);

		Ok(())
	}

	fn do_withdraw_liquidity(who: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(Self::pool_exists(pool_id), Error::<T, I>::PoolNotFound);

		let new_balance = Self::balance(pool_id)
			.checked_sub(amount)
			.ok_or(Error::<T, I>::NotEnoughBalance)?;

		// transfer amount to account
		T::LiquidityCurrency::transfer(&Self::account_id(), who, amount)?;

		// update balance
		Self::set_balance(pool_id, new_balance);

		Ok(())
	}

	fn do_set_identity(who: &T::AccountId, pool_id: LiquidityPoolId, identity_info: IdentityInfo) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, &who), Error::<T, I>::NoPermission);
		ensure!(
			identity_info.legal_name.len() <= 100
				&& identity_info.display_name.len() <= 200
				&& identity_info.web.len() <= 100
				&& identity_info.email.len() <= 50
				&& identity_info.image_url.len() <= 100,
			Error::<T, I>::IdentityInfoTooLong
		);

		if let Some((_, deposit_amount, _)) = Self::identity_infos(pool_id) {
			<IdentityInfos<T, I>>::insert(&pool_id, (identity_info, deposit_amount, false));
		} else {
			// reserve deposit from owner
			T::IdentityDepositCurrency::reserve(who, T::IdentityDeposit::get())?;

			<IdentityInfos<T, I>>::insert(&pool_id, (identity_info, T::IdentityDeposit::get(), false));
		}

		Ok(())
	}

	fn do_verify_identity(pool_id: LiquidityPoolId) -> DispatchResult {
		let (identity_info, deposit_amount, _) =
			Self::identity_infos(pool_id).ok_or(Error::<T, I>::IdentityInfoNotFound)?;
		<IdentityInfos<T, I>>::insert(&pool_id, (identity_info, deposit_amount, true));

		Ok(())
	}

	fn do_clear_identity(who: &T::AccountId, pool_id: LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, &who), Error::<T, I>::NoPermission);

		if let Some((_, deposit_amount, _)) = Self::identity_infos(pool_id) {
			T::IdentityDepositCurrency::unreserve(who, deposit_amount);
			<IdentityInfos<T, I>>::remove(&pool_id);
		}

		Ok(())
	}

	pub fn do_transfer_liquidity_pool(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		to: &T::AccountId,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, &who), Error::<T, I>::NoPermission);
		Self::do_clear_identity(&who, pool_id)?;

		let mut pool = Self::pools(pool_id).expect("is owner check ensures pool exist; qed");
		pool.owner = to.clone();
		<Pools<T, I>>::insert(&pool_id, pool);

		Ok(())
	}
}
