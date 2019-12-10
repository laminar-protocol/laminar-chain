#![cfg_attr(not(feature = "std"), no_std)]

mod liquidity_pool_option;
mod mock;
mod tests;

pub use liquidity_pool_option::LiquidityPoolOption;

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, dispatch::Result, ensure, traits::Get, Parameter,
};
use frame_system::{self as system, ensure_signed};
use orml_traits::MultiCurrency;
use primitives::Leverages;
use rstd::result;
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, One, SimpleArithmetic, Zero},
	Permill,
};
use traits::LiquidityPoolManager;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Self::Balance, CurrencyId = Self::CurrencyId>;
	type LiquidityPoolId: Parameter + Member + Copy + Ord + Default + SimpleArithmetic;
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize;
	type PoolManager: LiquidityPoolManager<Self::LiquidityPoolId>;
	type ExistentialDeposit: Get<Self::Balance>;
}

decl_storage! {
	trait Store for Module<T: Trait> as LiquidityPools {
		pub NextPoolId get(fn next_pool_id) build(|_| T::LiquidityPoolId::zero()): T::LiquidityPoolId;
		pub Owners get(fn owners): map T::LiquidityPoolId => Option<T::AccountId>;
		pub LiquidityPoolOptions get(fn liquidity_pool_options): double_map T::LiquidityPoolId, blake2_256(T::CurrencyId) => Option<LiquidityPoolOption<T::Balance>>;
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

		pub fn remove_pool(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId) -> Result {
			let who = ensure_signed(origin)?;
			Self::_remove_pool(who, pool_id, currency_id).map_err(|e| e.into())
		}

		pub fn deposit_liquidity(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, amount: T::Balance) -> Result {
			let who = ensure_signed(origin)?;
			Self::_deposit_liquidity(who, pool_id, currency_id, amount).map_err(|e| e.into())
		}

		pub fn withdraw_liquidity(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, amount: T::Balance) -> Result {
			let who = ensure_signed(origin)?;
			Self::_withdraw_liquidity(who, pool_id, currency_id, amount).map_err(|e| e.into())
		}

		pub fn set_spread(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, ask: Permill, bid: Permill) -> Result {
			let who = ensure_signed(origin)?;
			Self::_set_spread(who, pool_id, currency_id, ask, bid).map_err(|e| e.into())
		}

		pub fn set_additional_collateral_ratio(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, ratio: Option<Permill>) -> Result {
			let who = ensure_signed(origin)?;
			Self::_set_additional_collateral_ratio(who, pool_id, currency_id, ratio).map_err(|e| e.into())
		}

		pub fn set_enabled_trades(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, longs: Leverages, shorts: Leverages) -> Result {
			let who = ensure_signed(origin)?;
			Self::_set_enabled_trades(who, pool_id, currency_id, longs, shorts).map_err(|e| e.into())
		}
	}
}

decl_error! {
	// LiquidityPools module errors
	pub enum Error {
		NoPermission,
		CannotCreateMorePool,
		PoolNotFound,
		CannotRemovePool,
		DepositFailed,
		WithdrawFailed,
	}
}

impl<T: Trait> Module<T> {
	pub fn is_owner(pool_id: T::LiquidityPoolId, who: &T::AccountId) -> bool {
		match Self::owners(pool_id) {
			Some(id) => &id == who,
			None => false,
		}
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _create_pool(who: T::AccountId, currency_id: T::CurrencyId) -> result::Result<(), Error> {
		let pool_option: LiquidityPoolOption<T::Balance> = LiquidityPoolOption::default();

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
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);
		// TODO: Disable all tokens for this pool
		Ok(())
	}

	fn _remove_pool(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);
		ensure!(T::PoolManager::can_remove(pool_id), Error::CannotRemovePool);

		let pool = match <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id) {
			Some(pool) => pool,
			None => return Err(Error::PoolNotFound),
		};

		T::MultiCurrency::deposit(currency_id, &who, pool.balance).map_err(|e| e.into())?;
		<LiquidityPoolOptions<T>>::remove(pool_id, currency_id);
		Ok(())
	}

	fn _deposit_liquidity(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		amount: T::Balance,
	) -> result::Result<(), Error> {
		let mut pool = match <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id) {
			Some(pool) => pool,
			None => return Err(Error::PoolNotFound),
		};

		match pool.balance.checked_add(&amount) {
			Some(new_balance) => {
				T::MultiCurrency::withdraw(currency_id, &who, amount).map_err(|e| e.into())?;
				pool.balance = new_balance;
				<LiquidityPoolOptions<T>>::insert(pool_id, currency_id, pool);
				Ok(())
			}
			None => Err(Error::DepositFailed),
		}
	}

	fn _withdraw_liquidity(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		amount: T::Balance,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

		let mut pool = match <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id) {
			Some(pool) => pool,
			None => return Err(Error::PoolNotFound),
		};

		match pool.balance.checked_sub(&amount) {
			Some(new_balance) => {
				if new_balance < T::ExistentialDeposit::get() {
					return Err(Error::WithdrawFailed);
				}
				T::MultiCurrency::deposit(currency_id, &who, amount).map_err(|e| e.into())?;
				pool.balance = new_balance;
				<LiquidityPoolOptions<T>>::insert(pool_id, currency_id, pool);
				Ok(())
			}
			None => Err(Error::WithdrawFailed),
		}
	}

	fn _set_spread(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		ask: Permill,
		bid: Permill,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

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

	fn _set_additional_collateral_ratio(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		ratio: Option<Permill>,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

		match <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id) {
			Some(mut pool_option) => {
				pool_option.additional_collateral_ratio = ratio;
				<LiquidityPoolOptions<T>>::insert(pool_id, currency_id, pool_option);
				Ok(())
			}
			None => Err(Error::PoolNotFound),
		}
	}

	fn _set_enabled_trades(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		longs: Leverages,
		shorts: Leverages,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

		match <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id) {
			Some(mut pool_option) => {
				pool_option.enabled_longs = longs;
				pool_option.enabled_shorts = shorts;
				<LiquidityPoolOptions<T>>::insert(pool_id, currency_id, pool_option);
				Ok(())
			}
			None => Err(Error::PoolNotFound),
		}
	}
}
