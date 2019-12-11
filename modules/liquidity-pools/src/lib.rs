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
		<T as Trait>::CurrencyId,
		<T as Trait>::Balance,
	{
		/// Liquidity pool created (who, pool_id)
		LiquidityPoolOptionCreated(AccountId, LiquidityPoolId),
		/// Liquidity pool disabled (who, pool_id)
		LiquidityPoolDisabled(AccountId, LiquidityPoolId),
		/// Liquidity pool removed (who, pool_id, currency_id)
		LiquidityPoolRemoved(AccountId, LiquidityPoolId, CurrencyId),
		/// Deposit liquidity (who, pool_id, currency_id, amount)
		DepositLiquidity(AccountId, LiquidityPoolId, CurrencyId, Balance),
		/// Withdraw liquidity (who, pool_id, currency_id, amount)
		WithdrawLiquidity(AccountId, LiquidityPoolId, CurrencyId, Balance),
		/// Set spread (who, pool_id, currency_id, ask, bid)
		SetSpread(AccountId, LiquidityPoolId, CurrencyId, Permill, Permill),
		/// Set additional collateral ratio (who, pool_id, currency_id, ratio)
		SetAdditionalCollateralRatio(AccountId, LiquidityPoolId, CurrencyId, Option<Permill>),
		/// Set enabled trades (who, pool_id, currency_id, longs, shorts)
		SetEnabledTrades(AccountId, LiquidityPoolId, CurrencyId, Leverages, Leverages),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn create_pool(origin, currency_id: T::CurrencyId) -> Result {
			let who = ensure_signed(origin)?;
			Self::_create_pool(who, currency_id).map_err(|e| e.into())
		}

		pub fn disable_pool(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId) -> Result {
			let who = ensure_signed(origin)?;
			Self::_disable_pool(who, pool_id, currency_id).map_err(|e| e.into())
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
		let pool_id = Self::next_pool_id();
		// increment next pool id
		let next_pool_id = pool_id.checked_add(&One::one()).ok_or(Error::CannotCreateMorePool)?;
		<NextPoolId<T>>::put(next_pool_id);

		// create pool
		let pool_option: LiquidityPoolOption<T::Balance> = LiquidityPoolOption::default();
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool_option);

		// owner reference
		<Owners<T>>::insert(&pool_id, &who);

		Self::deposit_event(RawEvent::LiquidityPoolOptionCreated(who, pool_id));
		Ok(())
	}

	fn _disable_pool(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

		let mut pool = <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id).ok_or(Error::PoolNotFound)?;
		pool.enabled_longs = Leverages::none();
		pool.enabled_shorts = Leverages::none();
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);

		Self::deposit_event(RawEvent::LiquidityPoolDisabled(who, pool_id));
		Ok(())
	}

	fn _remove_pool(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);
		ensure!(T::PoolManager::can_remove(pool_id), Error::CannotRemovePool);

		let pool = <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id).ok_or(Error::PoolNotFound)?;

		T::MultiCurrency::deposit(currency_id, &who, pool.balance).map_err(|e| e.into())?;
		<LiquidityPoolOptions<T>>::remove(&pool_id, &currency_id);

		// remove owner reference
		<Owners<T>>::remove(&pool_id);

		Self::deposit_event(RawEvent::LiquidityPoolRemoved(who, pool_id, currency_id));
		Ok(())
	}

	fn _deposit_liquidity(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		amount: T::Balance,
	) -> result::Result<(), Error> {
		let mut pool = <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id).ok_or(Error::PoolNotFound)?;

		// update pool balance
		pool.balance = pool.balance.checked_add(&amount).ok_or(Error::DepositFailed)?;

		// withdraw account
		T::MultiCurrency::withdraw(currency_id, &who, amount).map_err(|e| e.into())?;

		// update pool balance
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);

		Self::deposit_event(RawEvent::DepositLiquidity(who, pool_id, currency_id, amount));
		Ok(())
	}

	fn _withdraw_liquidity(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		amount: T::Balance,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

		let mut pool = <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id).ok_or(Error::PoolNotFound)?;

		let new_balance = pool.balance.checked_sub(&amount).ok_or(Error::WithdrawFailed)?;

		// check minimum balance
		if new_balance < T::ExistentialDeposit::get() {
			return Err(Error::WithdrawFailed);
		}

		// deposit amount to account
		T::MultiCurrency::deposit(currency_id, &who, amount).map_err(|e| e.into())?;

		// update pool balance
		pool.balance = new_balance;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);

		Self::deposit_event(RawEvent::WithdrawLiquidity(who, pool_id, currency_id, amount));
		Ok(())
	}

	fn _set_spread(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		ask: Permill,
		bid: Permill,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

		let mut pool = <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id).ok_or(Error::PoolNotFound)?;
		pool.bid_spread = bid;
		pool.ask_spread = ask;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);

		Self::deposit_event(RawEvent::SetSpread(who, pool_id, currency_id, ask, bid));
		Ok(())
	}

	fn _set_additional_collateral_ratio(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		ratio: Option<Permill>,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

		let mut pool = <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id).ok_or(Error::PoolNotFound)?;
		pool.additional_collateral_ratio = ratio;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);

		Self::deposit_event(RawEvent::SetAdditionalCollateralRatio(who, pool_id, currency_id, ratio));
		Ok(())
	}

	fn _set_enabled_trades(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		longs: Leverages,
		shorts: Leverages,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, &who), Error::NoPermission);

		let mut pool = <LiquidityPoolOptions<T>>::get(&pool_id, &currency_id).ok_or(Error::PoolNotFound)?;
		pool.enabled_longs = longs;
		pool.enabled_shorts = shorts;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);

		Self::deposit_event(RawEvent::SetEnabledTrades(who, pool_id, currency_id, longs, shorts));
		Ok(())
	}
}
