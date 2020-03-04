#![cfg_attr(not(feature = "std"), no_std)]

mod liquidity_pool_option;
mod mock;
mod tests;

pub use liquidity_pool_option::LiquidityPoolOption;

use codec::{Decode, Encode, FullCodec};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter};
use frame_system::{self as system, ensure_root, ensure_signed};
use margin_protocol::TradingPair;
use orml_traits::{BasicCurrency, MultiCurrency};
use orml_utilities::Fixed128;
use primitives::{Leverage, Leverages};
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32Bit, CheckedAdd, CheckedSub, EnsureOrigin, MaybeSerializeDeserialize, Member, One,
	},
	DispatchResult, ModuleId, Permill, RuntimeDebug,
};
use sp_std::{prelude::*, result};
use traits::{LiquidityPoolManager, LiquidityPools, MarginProtocolLiquidityPools, SyntheticProtocolLiquidityPools};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct AccumulatedSwapRate {}

const MODULE_ID: ModuleId = ModuleId(*b"lami/lp_");

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Self::Balance, CurrencyId = Self::CurrencyId>;
	type LiquidityCurrency: BasicCurrency<Self::AccountId, Balance = Self::Balance>;
	type LiquidityPoolId: FullCodec
		+ Parameter
		+ Member
		+ Copy
		+ Ord
		+ Default
		+ AtLeast32Bit
		+ MaybeSerializeDeserialize;
	type Balance: Parameter + Member + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;
	type CurrencyId: FullCodec + Parameter + Member + Copy + MaybeSerializeDeserialize;
	type PoolManager: LiquidityPoolManager<Self::LiquidityPoolId, Self::Balance>;
	type ExistentialDeposit: Get<Self::Balance>;
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
	type MaxSwap: Get<Fixed128>;
}

decl_storage! {
	trait Store for Module<T: Trait> as LiquidityPools {
		pub NextPoolId get(fn next_pool_id): T::LiquidityPoolId;
		pub Owners get(fn owners): map hasher(blake2_256) T::LiquidityPoolId => Option<T::AccountId>;
		pub LiquidityPoolOptions get(fn liquidity_pool_options): double_map hasher(blake2_256) T::LiquidityPoolId, hasher(blake2_256) T::CurrencyId => Option<LiquidityPoolOption>;
		pub Balances get(fn balances): map hasher(blake2_256) T::LiquidityPoolId => T::Balance;
		pub MinAdditionalCollateralRatio get(fn min_additional_collateral_ratio) config(): Permill;
		pub SwapRates get(fn swap_rate): double_map hasher(blake2_256) T::LiquidityPoolId, hasher(blake2_256) (T::CurrencyId, T::CurrencyId) => Fixed128;
		pub AccumulatedSwapRates get(fn accumulated_swap_rate): double_map hasher(blake2_256) T::LiquidityPoolId, hasher(blake2_256) (T::CurrencyId, T::CurrencyId) => Fixed128;
		pub MaxSpread get(fn max_spread): map hasher(blake2_256) T::CurrencyId => Permill;
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
		/// Set min additional collateral ratio (min_additional_collateral_ratio)
		SetMinAdditionalCollateralRatio(Permill),
		/// Set synthetic enabled (who, pool_id, currency_id, enabled)
		SetSyntheticEnabled(AccountId, LiquidityPoolId, CurrencyId, bool),
		/// Swap rate updated (who, pool_id, currency_id, currency_id, rate)
		SwapRateUpdated(AccountId, LiquidityPoolId, CurrencyId, CurrencyId, Fixed128),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const ExistentialDeposit: T::Balance = T::ExistentialDeposit::get();

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

		pub fn deposit_liquidity(origin, pool_id: T::LiquidityPoolId, amount: T::Balance) {
			let who = ensure_signed(origin)?;
			Self::_deposit_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::DepositLiquidity(who, pool_id, amount));
		}

		pub fn withdraw_liquidity(origin, pool_id: T::LiquidityPoolId, amount: T::Balance) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);

			let new_balance = Self::balances(&pool_id).checked_sub(&amount).ok_or(Error::<T>::CannotWithdrawAmount)?;

			// check minimum balance
			if new_balance < T::ExistentialDeposit::get() {
				return Err(Error::<T>::CannotWithdrawExistentialDeposit.into());
			}

			Self::_withdraw_liquidity(&who, pool_id, amount)?;
			Self::deposit_event(RawEvent::WithdrawLiquidity(who, pool_id, amount));
		}

		pub fn set_spread(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, ask: Permill, bid: Permill) {
			let who = ensure_signed(origin)?;
			Self::_set_spread(&who, pool_id, currency_id, ask, bid)?;
			Self::deposit_event(RawEvent::SetSpread(who, pool_id, currency_id, ask, bid));
		}

		pub fn set_additional_collateral_ratio(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, ratio: Option<Permill>) {
			let who = ensure_signed(origin)?;
			Self::_set_additional_collateral_ratio(&who, pool_id, currency_id, ratio)?;
			Self::deposit_event(RawEvent::SetAdditionalCollateralRatio(who, pool_id, currency_id, ratio));
		}

		pub fn set_enabled_trades(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, enabled: Leverages) {
			let who = ensure_signed(origin)?;
			Self::_set_enabled_trades(&who, pool_id, currency_id, enabled)?;
			Self::deposit_event(RawEvent::SetEnabledTrades(who, pool_id, currency_id, enabled));
		}

		pub fn set_min_additional_collateral_ratio(origin, ratio: Permill) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			MinAdditionalCollateralRatio::put(ratio);
			Self::deposit_event(RawEvent::SetMinAdditionalCollateralRatio(ratio));
		}

		pub fn set_synthetic_enabled(origin, pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, enabled: bool) {
			let who = ensure_signed(origin)?;
			Self::_set_synthetic_enabled(&who, pool_id, currency_id, enabled)?;
			Self::deposit_event(RawEvent::SetSyntheticEnabled(who, pool_id, currency_id, enabled));
		}

		pub fn update_swap(origin, pool_id: T::LiquidityPoolId, pair: (T::CurrencyId, T::CurrencyId), rate: Fixed128) {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_owner(pool_id, &who), Error::<T>::NoPermission);
			ensure!(rate <= T::MaxSwap::get(), Error::<T>::SwapRateTooHigh);
			<SwapRates<T>>::insert(pool_id, pair, rate);
			Self::deposit_event(RawEvent::SwapRateUpdated(who, pool_id, pair.0, pair.1, rate));
		}
	}
}

decl_error! {
	// LiquidityPools module errors
	pub enum Error for Module<T: Trait> {
		NoPermission,
		CannotCreateMorePool,
		CannotRemovePool,
		CannotDepositAmount,
		CannotWithdrawAmount,
		CannotWithdrawExistentialDeposit,
		SwapRateTooHigh,
	}
}

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn is_owner(pool_id: T::LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::owners(pool_id).map_or(false, |id| &id == who)
	}

	pub fn is_enabled(pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId, leverage: Leverage) -> bool {
		Self::liquidity_pool_options(&pool_id, &currency_id)
			.map_or(false, |pool| pool.enabled_trades.contains(leverage))
	}
}

impl<T: Trait> LiquidityPools<T::AccountId> for Module<T> {
	type LiquidityPoolId = T::LiquidityPoolId;
	type CurrencyId = T::CurrencyId;
	type Balance = T::Balance;

	fn get_bid_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id).map(|pool| pool.bid_spread)
	}

	fn get_ask_spread(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id).map(|pool| pool.ask_spread)
	}

	fn ensure_liquidity(_pool_id: Self::LiquidityPoolId) -> bool {
		unimplemented!()
	}

	fn is_owner(pool_id: Self::LiquidityPoolId, who: &T::AccountId) -> bool {
		Self::is_owner(pool_id, who)
	}

	fn is_allowed_position(pool_id: Self::LiquidityPoolId, currency_id: Self::CurrencyId, leverage: Leverage) -> bool {
		Self::is_enabled(pool_id, currency_id, leverage)
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

type TradingPairOf<T> = TradingPair<<T as Trait>::CurrencyId>;

impl<T: Trait> MarginProtocolLiquidityPools<T::AccountId> for Module<T> {
	type TradingPair = TradingPairOf<T>;
	fn get_swap_rate(_pair: Self::TradingPair) -> Fixed128 {
		unimplemented!()
	}

	fn get_accumulated_swap_rate(_pair: Self::TradingPair) -> Fixed128 {
		unimplemented!()
	}

	fn can_open_position(_pair: Self::TradingPair, _leverage: Leverage, _leveraged_amount: Self::Balance) -> bool {
		unimplemented!()
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
		<Owners<T>>::insert(&pool_id, who);
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

	fn _deposit_liquidity(who: &T::AccountId, pool_id: T::LiquidityPoolId, amount: T::Balance) -> DispatchResult {
		let balance = Self::balances(&pool_id);
		let new_balance = balance.checked_add(&amount).ok_or(Error::<T>::CannotDepositAmount)?;
		// transfer amount to this pool
		T::LiquidityCurrency::transfer(who, &Self::account_id(), amount)?;
		// update balance
		<Balances<T>>::insert(&pool_id, new_balance);
		Ok(())
	}

	fn _withdraw_liquidity(who: &T::AccountId, pool_id: T::LiquidityPoolId, amount: T::Balance) -> DispatchResult {
		let new_balance = Self::balances(&pool_id)
			.checked_sub(&amount)
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
		currency_id: T::CurrencyId,
		ask: Permill,
		bid: Permill,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
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
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
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
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.enabled_trades = enabled;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);
		Ok(())
	}

	fn _set_synthetic_enabled(
		who: &T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		enabled: bool,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.synthetic_enabled = enabled;
		<LiquidityPoolOptions<T>>::insert(&pool_id, &currency_id, pool);
		Ok(())
	}
}
