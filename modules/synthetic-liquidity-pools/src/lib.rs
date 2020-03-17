#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use codec::{Decode, Encode, FullCodec};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter};
use frame_system::{self as system, ensure_root, ensure_signed};
use orml_traits::{BasicCurrency, MultiCurrency};
use primitives::{Balance, CurrencyId};
use sp_runtime::{
	traits::{AccountIdConversion, AtLeast32Bit, CheckedAdd, EnsureOrigin, MaybeSerializeDeserialize, Member, One},
	DispatchResult, ModuleId, PerThing, Permill, RuntimeDebug,
};
use sp_std::{prelude::*, result};
use traits::{LiquidityPoolManager, LiquidityPools, SyntheticProtocolLiquidityPools};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct SyntheticLiquidityPoolOption {
	pub bid_spread: Permill,
	pub ask_spread: Permill,
	pub additional_collateral_ratio: Option<Permill>,
	pub synthetic_enabled: bool,
}

const MODULE_ID: ModuleId = ModuleId(*b"lami/slp");

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Balance>;
	type LiquidityPoolId: FullCodec
		+ Parameter
		+ Member
		+ Copy
		+ Ord
		+ Default
		+ AtLeast32Bit
		+ MaybeSerializeDeserialize;
	type PoolManager: LiquidityPoolManager<Self::LiquidityPoolId, Balance>;
	type ExistentialDeposit: Get<Balance>;
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
}

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticLiquidityPools {
		pub NextPoolId get(fn next_pool_id): T::LiquidityPoolId;
		pub Owners get(fn owners): map hasher(blake2_256) T::LiquidityPoolId => Option<(T::AccountId, T::LiquidityPoolId)>;
		pub LiquidityPoolOptions get(fn liquidity_pool_options): double_map hasher(blake2_256) T::LiquidityPoolId, hasher(blake2_256) CurrencyId => Option<SyntheticLiquidityPoolOption>;
		pub Balances get(fn balances): map hasher(blake2_256) T::LiquidityPoolId => Balance;
		pub MinAdditionalCollateralRatio get(fn min_additional_collateral_ratio) config(): Permill;
		pub MaxSpread get(fn max_spread): map hasher(blake2_256) CurrencyId => Permill;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::LiquidityPoolId,
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
		/// Set spread (who, pool_id, currency_id, bid, ask)
		SetSpread(AccountId, LiquidityPoolId, CurrencyId, Permill, Permill),
		/// Set additional collateral ratio (who, pool_id, currency_id, ratio)
		SetAdditionalCollateralRatio(AccountId, LiquidityPoolId, CurrencyId, Option<Permill>),
		/// Set min additional collateral ratio (min_additional_collateral_ratio)
		SetMinAdditionalCollateralRatio(Permill),
		/// Set synthetic enabled (who, pool_id, currency_id, enabled)
		SetSyntheticEnabled(AccountId, LiquidityPoolId, CurrencyId, bool),
		/// Max spread updated (currency_id, spread)
		MaxSpreadUpdated(CurrencyId, Permill),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const ExistentialDeposit: Balance = T::ExistentialDeposit::get();

		pub fn create_pool(origin) {
			let who = ensure_signed(origin)?;
			let pool_id = Self::_create_pool(&who)?;
			Self::deposit_event(RawEvent::LiquidityPoolCreated(who, pool_id));
		}

		pub fn disable_pool(origin, pool_id: T::LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_disable_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolDisabled(who, pool_id));
		}

		pub fn remove_pool(origin, pool_id: T::LiquidityPoolId) {
			let who = ensure_signed(origin)?;
			Self::_remove_pool(&who, pool_id)?;
			Self::deposit_event(RawEvent::LiquidityPoolRemoved(who, pool_id));
		}

		pub fn deposit_liquidity(origin, pool_id: T::LiquidityPoolId, amount: Balance) {
			let who = ensure_signed(origin)?;
			Self::_deposit_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::DepositLiquidity(who, pool_id, amount));
		}

		pub fn withdraw_liquidity(origin, pool_id: T::LiquidityPoolId, amount: Balance) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

			let new_balance = Self::balances(&pool_id).checked_sub(amount).ok_or(Error::<T>::CannotWithdrawAmount)?;

			// check minimum balance
			if new_balance < T::ExistentialDeposit::get() {
				return Err(Error::<T>::CannotWithdrawExistentialDeposit.into());
			}

			Self::_withdraw_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::WithdrawLiquidity(who, pool_id, amount));
		}

		pub fn set_spread(origin, pool_id: T::LiquidityPoolId, currency_id: CurrencyId, bid: Permill, ask: Permill) {
			let who = ensure_signed(origin)?;
			Self::_set_spread(&who, pool_id, currency_id, bid, ask)?;
			Self::deposit_event(RawEvent::SetSpread(who, pool_id, currency_id, bid, ask));
		}

		pub fn set_additional_collateral_ratio(origin, pool_id: T::LiquidityPoolId, currency_id: CurrencyId, ratio: Option<Permill>) {
			let who = ensure_signed(origin)?;
			Self::_set_additional_collateral_ratio(&who, pool_id, currency_id, ratio)?;
			Self::deposit_event(RawEvent::SetAdditionalCollateralRatio(who, pool_id, currency_id, ratio));
		}

		pub fn set_min_additional_collateral_ratio(origin, ratio: Permill) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			MinAdditionalCollateralRatio::put(ratio);
			Self::deposit_event(RawEvent::SetMinAdditionalCollateralRatio(ratio));
		}

		pub fn set_synthetic_enabled(origin, pool_id: T::LiquidityPoolId, currency_id: CurrencyId, enabled: bool) {
			let who = ensure_signed(origin)?;
			Self::_set_synthetic_enabled(&who, pool_id, currency_id, enabled)?;
			Self::deposit_event(RawEvent::SetSyntheticEnabled(who, pool_id, currency_id, enabled));
		}

		pub fn set_max_spread(origin, currency_id: CurrencyId, max_spread: Permill) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			MaxSpread::insert(currency_id, max_spread);
			Self::deposit_event(RawEvent::MaxSpreadUpdated(currency_id, max_spread));
		}
	}
}

decl_error! {
	// SyntheticLiquidityPools module errors
	pub enum Error for Module<T: Trait> {
		NoPermission,
		CannotCreateMorePool,
		CannotRemovePool,
		CannotDepositAmount,
		CannotWithdrawAmount,
		CannotWithdrawExistentialDeposit,
		SpreadTooHigh,
		PoolNotFound,
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn is_owner(pool_id: T::LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::owners(pool_id).map_or(false, |(id, _)| &id == who)
	}
}

impl<T: Trait> LiquidityPools<T::AccountId> for Module<T> {
	type LiquidityPoolId = T::LiquidityPoolId;
	type CurrencyId = CurrencyId;
	type Balance = Balance;

	fn ensure_liquidity(pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> DispatchResult {
		unimplemented!()
	}

	fn is_owner(pool_id: Self::LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::is_owner(pool_id, who)
	}

	/// Check collateral balance of `pool_id`.
	fn liquidity(pool_id: Self::LiquidityPoolId) -> Self::Balance {
		Self::balances(&pool_id)
	}

	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(
		source: &T::AccountId,
		pool_id: Self::LiquidityPoolId,
		amount: Self::Balance,
	) -> DispatchResult {
		Self::_deposit_liquidity(source, pool_id, amount)
	}

	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(
		dest: &T::AccountId,
		pool_id: Self::LiquidityPoolId,
		amount: Self::Balance,
	) -> DispatchResult {
		Self::_withdraw_liquidity(dest, pool_id, amount)
	}
}

impl<T: Trait> SyntheticProtocolLiquidityPools<T::AccountId> for Module<T> {
	fn get_bid_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id).map(|pool| pool.bid_spread)
	}

	fn get_ask_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id).map(|pool| pool.ask_spread)
	}

	fn get_additional_collateral_ratio(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Permill {
		let min_ratio = Self::min_additional_collateral_ratio();

		Self::liquidity_pool_options(&pool_id, &currency_id).map_or(min_ratio, |pool| {
			pool.additional_collateral_ratio.unwrap_or(min_ratio).max(min_ratio)
		})
	}

	fn can_mint(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> bool {
		Self::liquidity_pool_options(&pool_id, &currency_id).map_or(false, |pool| pool.synthetic_enabled)
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _create_pool(who: &T::AccountId) -> result::Result<T::LiquidityPoolId, Error<T>> {
		let pool_id = Self::next_pool_id();
		// increment next pool id
		let next_pool_id = pool_id
			.checked_add(&One::one())
			.ok_or(Error::<T>::CannotCreateMorePool)?;
		<NextPoolId<T>>::put(next_pool_id);
		// owner reference
		<Owners<T>>::insert(&pool_id, (who, pool_id));
		Ok(pool_id)
	}

	fn _disable_pool(who: &T::AccountId, pool_id: T::LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		<LiquidityPoolOptions<T>>::remove_prefix(&pool_id);
		Ok(())
	}

	fn _remove_pool(who: &T::AccountId, pool_id: T::LiquidityPoolId) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		ensure!(T::PoolManager::can_remove(pool_id), Error::<T>::CannotRemovePool);

		let balance = Self::balances(&pool_id);
		// transfer balance to pool owner
		T::LiquidityCurrency::transfer(&Self::account_id(), who, balance)?;

		<Balances<T>>::remove(&pool_id);
		<Owners<T>>::remove(&pool_id);
		<LiquidityPoolOptions<T>>::remove_prefix(&pool_id);

		Ok(())
	}

	fn _deposit_liquidity(who: &T::AccountId, pool_id: T::LiquidityPoolId, amount: Balance) -> DispatchResult {
		ensure!(<Owners<T>>::contains_key(&pool_id), Error::<T>::PoolNotFound);
		let balance = Self::balances(&pool_id);
		let new_balance = balance.checked_add(amount).ok_or(Error::<T>::CannotDepositAmount)?;
		// transfer amount to this pool
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount)?;
		// update balance
		<Balances<T>>::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _withdraw_liquidity(who: &T::AccountId, pool_id: T::LiquidityPoolId, amount: Balance) -> DispatchResult {
		let new_balance = Self::balances(&pool_id)
			.checked_sub(amount)
			.ok_or(Error::<T>::CannotWithdrawAmount)?;

		// transfer amount to account
		T::LiquidityCurrency::transfer(&Self::account_id(), who, amount)?;

		// update balance
		<Balances<T>>::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _set_spread(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: CurrencyId,
		bid: Permill,
		ask: Permill,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let max_spread = Self::max_spread(&currency_id);
		if !max_spread.is_zero() {
			ensure!(ask <= max_spread && bid <= max_spread, Error::<T>::SpreadTooHigh);
		}
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.bid_spread = bid;
		pool.ask_spread = ask;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);
		Ok(())
	}

	fn _set_additional_collateral_ratio(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: CurrencyId,
		ratio: Option<Permill>,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.additional_collateral_ratio = ratio;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);
		Ok(())
	}

	fn _set_synthetic_enabled(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: CurrencyId,
		enabled: bool,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.synthetic_enabled = enabled;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);
		Ok(())
	}
}
