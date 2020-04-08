#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure};
use frame_system::{self as system, ensure_root, ensure_signed};
use orml_traits::MultiCurrency;
use primitives::{Balance, CurrencyId, LiquidityPoolId};
use sp_runtime::{traits::EnsureOrigin, DispatchResult, ModuleId, PerThing, Permill, RuntimeDebug};
use sp_std::prelude::*;
use traits::{LiquidityPools, OnDisableLiquidityPool, OnRemoveLiquidityPool, SyntheticProtocolLiquidityPools};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct SyntheticLiquidityPoolOption {
	pub bid_spread: Permill,
	pub ask_spread: Permill,
	pub additional_collateral_ratio: Option<Permill>,
	pub synthetic_enabled: bool,
}

pub const MODULE_ID: ModuleId = ModuleId(*b"lami/slp");

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type BaseLiquidityPools: LiquidityPools<Self::AccountId>;
	type MultiCurrency: MultiCurrency<Self::AccountId, Balance = Balance, CurrencyId = CurrencyId>;
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
}

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticLiquidityPools {
		pub LiquidityPoolOptions get(fn liquidity_pool_options): double_map hasher(blake2_128_concat) LiquidityPoolId, hasher(blake2_128_concat) CurrencyId => Option<SyntheticLiquidityPoolOption>;
		pub MinAdditionalCollateralRatio get(fn min_additional_collateral_ratio) config(): Permill;
		pub MaxSpread get(fn max_spread): map hasher(blake2_128_concat) CurrencyId => Permill;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
	{
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

		pub fn set_spread(origin, pool_id: LiquidityPoolId, currency_id: CurrencyId, bid: Permill, ask: Permill) {
			let who = ensure_signed(origin)?;
			Self::_set_spread(&who, pool_id, currency_id, bid, ask)?;
			Self::deposit_event(RawEvent::SetSpread(who, pool_id, currency_id, bid, ask));
		}

		pub fn set_additional_collateral_ratio(origin, pool_id: LiquidityPoolId, currency_id: CurrencyId, ratio: Option<Permill>) {
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

		pub fn set_synthetic_enabled(origin, pool_id: LiquidityPoolId, currency_id: CurrencyId, enabled: bool) {
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
		SpreadTooHigh,
	}
}

impl<T: Trait> LiquidityPools<T::AccountId> for Module<T> {
	fn all() -> Vec<LiquidityPoolId> {
		T::BaseLiquidityPools::all()
	}

	fn is_owner(pool_id: LiquidityPoolId, who: &T::AccountId) -> bool {
		T::BaseLiquidityPools::is_owner(pool_id, who)
	}

	/// Check collateral balance of `pool_id`.
	fn liquidity(pool_id: LiquidityPoolId) -> Balance {
		T::BaseLiquidityPools::liquidity(pool_id)
	}

	/// Deposit some amount of collateral to `pool_id`, from `source`.
	fn deposit_liquidity(source: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		T::BaseLiquidityPools::deposit_liquidity(source, pool_id, amount)
	}

	/// Withdraw some amount of collateral to `dest`, from `pool_id`.
	fn withdraw_liquidity(dest: &T::AccountId, pool_id: LiquidityPoolId, amount: Balance) -> DispatchResult {
		T::BaseLiquidityPools::withdraw_liquidity(dest, pool_id, amount)
	}
}

impl<T: Trait> SyntheticProtocolLiquidityPools<T::AccountId> for Module<T> {
	fn get_bid_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id).map(|pool| pool.bid_spread)
	}

	fn get_ask_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Permill> {
		Self::liquidity_pool_options(&pool_id, &currency_id).map(|pool| pool.ask_spread)
	}

	fn get_additional_collateral_ratio(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Permill {
		let min_ratio = Self::min_additional_collateral_ratio();

		Self::liquidity_pool_options(&pool_id, &currency_id).map_or(min_ratio, |pool| {
			pool.additional_collateral_ratio.unwrap_or(min_ratio).max(min_ratio)
		})
	}

	fn can_mint(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> bool {
		Self::liquidity_pool_options(&pool_id, &currency_id).map_or(false, |pool| pool.synthetic_enabled)
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _set_spread(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
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
		LiquidityPoolOptions::insert(&pool_id, &currency_id, pool);
		Ok(())
	}

	fn _set_additional_collateral_ratio(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		ratio: Option<Permill>,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.additional_collateral_ratio = ratio;
		LiquidityPoolOptions::insert(&pool_id, &currency_id, pool);
		Ok(())
	}

	fn _set_synthetic_enabled(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		enabled: bool,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let mut pool = Self::liquidity_pool_options(&pool_id, &currency_id).unwrap_or_default();
		pool.synthetic_enabled = enabled;
		LiquidityPoolOptions::insert(&pool_id, &currency_id, pool);
		Ok(())
	}
}

impl<T: Trait> OnDisableLiquidityPool for Module<T> {
	fn on_disable(pool_id: LiquidityPoolId) {
		LiquidityPoolOptions::remove_prefix(&pool_id);
	}
}

impl<T: Trait> OnRemoveLiquidityPool for Module<T> {
	fn on_remove(pool_id: LiquidityPoolId) {
		LiquidityPoolOptions::remove_prefix(&pool_id);
	}
}
