#![cfg_attr(not(feature = "std"), no_std)]

mod liquidity_pool_option;
mod mock;
mod tests;

pub use liquidity_pool_option::LiquidityPoolOption;

use codec::FullCodec;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, dispatch::Result, ensure, traits::Get, Parameter,
};
use frame_system::{self as system, ensure_signed};
use orml_traits::{BasicCurrency, MultiCurrency};
use primitives::{Leverage, Leverages};
use rstd::prelude::*;
use rstd::result;
use sp_runtime::{
	traits::{
		AccountIdConversion, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, One, SimpleArithmetic, Zero,
	},
	ModuleId, Permill,
};
use traits::{
	LiquidityPoolBaseTypes, LiquidityPoolManager, LiquidityPools, LiquidityPoolsConfig, LiquidityPoolsCurrency,
	LiquidityPoolsPosition,
};

const MODULE_ID: ModuleId = ModuleId(*b"flow/lp_");

type ErrorOf<T> = <<T as Trait>::MultiCurrency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::Error;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Self::Balance, CurrencyId = Self::CurrencyId>;
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Self::Balance, Error = ErrorOf<Self>>;
	type LiquidityPoolId: FullCodec
		+ Parameter
		+ Member
		+ Copy
		+ Ord
		+ Default
		+ SimpleArithmetic
		+ MaybeSerializeDeserialize;
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type CurrencyId: FullCodec + Parameter + Member + Copy + MaybeSerializeDeserialize;
	type PoolManager: LiquidityPoolManager<Self::LiquidityPoolId>;
	type ExistentialDeposit: Get<Self::Balance>;
	type LiquidityCurrencyIds: Get<Vec<Self::CurrencyId>>;
}

decl_storage! {
	trait Store for Module<T: Trait> as LiquidityPools {
		pub NextPoolId get(fn next_pool_id) build(|_| T::LiquidityPoolId::zero()): T::LiquidityPoolId;
		pub Owners get(fn owners): map T::LiquidityPoolId => Option<T::AccountId>;
		pub LiquidityPoolOptions get(fn liquidity_pool_options): double_map T::LiquidityPoolId, blake2_256(T::CurrencyId) => Option<LiquidityPoolOption>;
		pub Balances get(fn balances): map T::LiquidityPoolId => T::Balance;
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
		LiquidityPoolCreated(AccountId, LiquidityPoolId),
		/// Liquidity pool disabled (who, pool_id)
		LiquidityPoolDisabled(AccountId, LiquidityPoolId),
		/// Liquidity pool removed (who, pool_id)
		LiquidityPoolRemoved(AccountId, LiquidityPoolId),
		/// Deposit liquidity (who, pool_id, amount)
		DepositLiquidity(AccountId, LiquidityPoolId, Balance),
		/// Withdraw liquidity (who, pool_id, amount)
		WithdrawLiquidity(AccountId, LiquidityPoolId, Balance),
		/// Set spread (who, pool_id, currency_id, ask, bid)
		SetSpread(AccountId, LiquidityPoolId, CurrencyId, Permill, Permill),
		/// Set additional collateral ratio (who, pool_id, currency_id, ratio)
		SetAdditionalCollateralRatio(AccountId, LiquidityPoolId, CurrencyId, Option<Permill>),
		/// Set enabled trades (who, pool_id, currency_id, enabled)
		SetEnabledTrades(AccountId, LiquidityPoolId, CurrencyId, Leverages),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn create_pool(origin) -> Result {
			let who = ensure_signed(origin)?;
			let pool_id = Self::_create_pool(&who)?;
			Self::deposit_event(RawEvent::LiquidityPoolCreated(who, pool_id));
			Ok(())
		}

		pub fn disable_pool(origin, pool_id: T::LiquidityPoolId) -> Result {
			let who = ensure_signed(origin)?;
			Self::_disable_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolDisabled(who, pool_id));
			Ok(())
		}

		pub fn remove_pool(origin, pool_id: T::LiquidityPoolId) -> Result {
			let who = ensure_signed(origin)?;
			Self::_remove_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolRemoved(who, pool_id));
			Ok(())
		}

		pub fn deposit_liquidity(origin, pool_id: T::LiquidityPoolId, amount: T::Balance) -> Result {
			let who = ensure_signed(origin)?;
			Self::_deposit_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::DepositLiquidity(who, pool_id, amount));
			Ok(())
		}

		pub fn withdraw_liquidity(origin, pool_id: T::LiquidityPoolId, amount: T::Balance) -> Result {
			let who = ensure_signed(origin)?;
			Self::_withdraw_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::WithdrawLiquidity(who, pool_id, amount));
			Ok(())
		}

		pub fn set_spread(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, ask: Permill, bid: Permill) -> Result {
			let who = ensure_signed(origin)?;
			Self::_set_spread(&who, pool_id, currency_id, ask, bid)?;
			Self::deposit_event(RawEvent::SetSpread(who, pool_id, currency_id, ask, bid));
			Ok(())
		}

		pub fn set_additional_collateral_ratio(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, ratio: Option<Permill>) -> Result {
			let who = ensure_signed(origin)?;
			Self::_set_additional_collateral_ratio(&who, pool_id, currency_id, ratio)?;
			Self::deposit_event(RawEvent::SetAdditionalCollateralRatio(who, pool_id, currency_id, ratio));
			Ok(())
		}

		pub fn set_enabled_trades(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, enabled: Leverages) -> Result {
			let who = ensure_signed(origin)?;
			Self::_set_enabled_trades(&who, pool_id, currency_id, enabled)?;
			Self::deposit_event(RawEvent::SetEnabledTrades(who, pool_id, currency_id, enabled));
			Ok(())
		}
	}
}

decl_error! {
	// LiquidityPools module errors
	pub enum Error {
		NoPermission,
		CannotCreateMorePool,
		CannotRemovePool,
		CannotDepositAmount,
		CannotWithdrawAmount,
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn is_owner(pool_id: T::LiquidityPoolId, who: &T::AccountId) -> bool {
		match Self::owners(pool_id) {
			Some(id) => &id == who,
			None => false,
		}
	}

	pub fn is_enabled(pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, leverage: Leverage) -> bool {
		match Self::liquidity_pool_options(&pool_id, &currency_id) {
			Some(pool) => pool.enabled.contains(leverage),
			None => false,
		}
	}
}

impl<T: Trait> LiquidityPoolBaseTypes for Module<T> {
	type LiquidityPoolId = T::LiquidityPoolId;
	type CurrencyId = T::CurrencyId;
}

impl<T: Trait> LiquidityPoolsConfig<T::AccountId> for Module<T> {
	fn get_bid_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id).map(|pool| pool.bid_spread)
	}

	fn get_ask_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id).map(|pool| pool.ask_spread)
	}

	fn get_additional_collateral_ratio(
		pool_id: Self::LiquidityPoolId,
		currency_id: Self::CurrencyId,
	) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id)
			.map(|pool| pool.additional_collateral_ratio)
			.unwrap_or(None)
	}

	fn is_owner(pool_id: Self::LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::is_owner(pool_id, who)
	}
}

impl<T: Trait> LiquidityPoolsPosition for Module<T> {
	fn is_allowed_position(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId, leverage: Leverage) -> bool {
		Self::is_enabled(pool_id, currency_id, leverage)
	}
}

impl<T: Trait> LiquidityPoolsCurrency<T::AccountId> for Module<T> {
	type Balance = T::Balance;
	type Error = Error;

	/// Check collateral balance of `pool_id`.
	fn balance(pool_id: Self::LiquidityPoolId) -> Self::Balance {
		Self::balances(&pool_id)
	}

	/// Deposit some amount of collateral to `pool_id`, from `who`.
	fn deposit(
		from: &T::AccountId,
		pool_id: Self::LiquidityPoolId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error> {
		Self::_deposit_liquidity(from, pool_id, amount)
	}

	/// Withdraw some amount of collateral to `who`, from `pool_id`.
	fn withdraw(
		to: &T::AccountId,
		pool_id: Self::LiquidityPoolId,
		amount: Self::Balance,
	) -> result::Result<(), Self::Error> {
		Self::_withdraw_liquidity(to, pool_id, amount)
	}
}

impl<T: Trait> LiquidityPools<T::AccountId> for Module<T> {}

// Private methods
impl<T: Trait> Module<T> {
	fn _create_pool(who: &T::AccountId) -> result::Result<T::LiquidityPoolId, Error> {
		let pool_id = Self::next_pool_id();
		// increment next pool id
		let next_pool_id = pool_id.checked_add(&One::one()).ok_or(Error::CannotCreateMorePool)?;
		<NextPoolId<T>>::put(next_pool_id);
		// owner reference
		<Owners<T>>::insert(&pool_id, who);
		Ok(pool_id)
	}

	fn _disable_pool(who: &T::AccountId, pool_id: T::LiquidityPoolId) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, who), Error::NoPermission);

		for currency_id in T::LiquidityCurrencyIds::get() {
			if let Some(mut pool) = Self::liquidity_pool_options(&pool_id, currency_id) {
				pool.enabled = Leverages::none();
				<LiquidityPoolOptions<T>>::insert(&pool_id, currency_id, pool);
			}
		}

		Ok(())
	}

	fn _remove_pool(who: &T::AccountId, pool_id: T::LiquidityPoolId) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, who), Error::NoPermission);
		ensure!(T::PoolManager::can_remove(pool_id), Error::CannotRemovePool);

		let balance = Self::balances(&pool_id);
		// transfer balance to pool owner
		T::LiquidityCurrency::transfer(&Self::account_id(), who, balance).map_err(|e| e.into())?;

		<Balances<T>>::remove(&pool_id);
		<Owners<T>>::remove(&pool_id);

		for currency_id in T::LiquidityCurrencyIds::get() {
			<LiquidityPoolOptions<T>>::remove(&pool_id, currency_id);
		}

		Ok(())
	}

	fn _deposit_liquidity(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		amount: T::Balance,
	) -> result::Result<(), Error> {
		let balance = Self::balances(&pool_id);
		let new_balance = balance.checked_add(&amount).ok_or(Error::CannotDepositAmount)?;
		// transfer amount to this pool
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount).map_err(|e| e.into())?;
		// update balance
		<Balances<T>>::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _withdraw_liquidity(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		amount: T::Balance,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, who), Error::NoPermission);
		let balance = Self::balances(&pool_id);
		let new_balance = balance.checked_sub(&amount).ok_or(Error::CannotWithdrawAmount)?;

		// check minimum balance
		if new_balance < T::ExistentialDeposit::get() {
			return Err(Error::CannotWithdrawAmount);
		}

		// transfer amount to account
		T::LiquidityCurrency::transfer(&Self::account_id(), who, amount).map_err(|e| e.into())?;

		// update balance
		<Balances<T>>::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _set_spread(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		ask: Permill,
		bid: Permill,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, who), Error::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.bid_spread = bid;
		pool.ask_spread = ask;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);
		Ok(())
	}

	fn _set_additional_collateral_ratio(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		ratio: Option<Permill>,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, who), Error::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.additional_collateral_ratio = ratio;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);
		Ok(())
	}

	fn _set_enabled_trades(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		enabled: Leverages,
	) -> result::Result<(), Error> {
		ensure!(Self::is_owner(pool_id, who), Error::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.enabled = enabled;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);
		Ok(())
	}
}
