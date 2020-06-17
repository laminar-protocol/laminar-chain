#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::EnsureOrigin};
use frame_system::{self as system, ensure_root, ensure_signed};
use primitives::{Balance, CurrencyId, LiquidityPoolId};
use sp_runtime::{traits::Zero, DispatchResult, ModuleId, Permill, RuntimeDebug};
use sp_std::prelude::*;
use traits::{LiquidityPools, OnDisableLiquidityPool, OnRemoveLiquidityPool, SyntheticProtocolLiquidityPools};

/// Currency option in a pool of synthetic.
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct SyntheticPoolCurrencyOption {
	/// Bid spread.
	///
	/// DEFAULT-NOTE: `None`, pool owner must set spread.
	pub bid_spread: Option<Balance>,

	/// Ask spread.
	///
	/// DEFAULT-NOTE: `None`, pool owner must set spread.
	pub ask_spread: Option<Balance>,

	/// Additional collateral ratio.
	///
	/// DEFAULT-NOTE: `None`. If not set or smaller than min additional swap rate, min value will be used instead.
	pub additional_collateral_ratio: Option<Permill>,

	/// Is a synthetic currency enabled to mint in the pool.
	///
	/// DEFAULT-NOTE: default not enabled.
	pub synthetic_enabled: bool,
}

pub const MODULE_ID: ModuleId = ModuleId(*b"lami/slp");

pub trait Trait: frame_system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The `LiquidityPools` implementation.
	type BaseLiquidityPools: LiquidityPools<Self::AccountId>;

	/// Required origin for updating protocol options.
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
}

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticLiquidityPools {
		/// Currency options in a liquidity pool.
		pub PoolCurrencyOptions get(fn pool_currency_options): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) CurrencyId => SyntheticPoolCurrencyOption;

		/// Minimum additional collateral ratio.
		pub MinAdditionalCollateralRatio get(fn min_additional_collateral_ratio) config(): Permill;

		/// Maximum spread of a currency.
		pub MaxSpread get(fn max_spread): map hasher(twox_64_concat) CurrencyId => Balance;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
	{
		/// Spread set: (who, pool_id, currency_id, bid, ask)
		SpreadSet(AccountId, LiquidityPoolId, CurrencyId, Balance, Balance),

		/// Additional collateral ratio set: (who, pool_id, currency_id, ratio)
		AdditionalCollateralRatioSet(AccountId, LiquidityPoolId, CurrencyId, Option<Permill>),

		/// Min additional collateral ratio set: (min_additional_collateral_ratio)
		MinAdditionalCollateralRatioSet(Permill),

		/// Synthetic enabled set: (who, pool_id, currency_id, is_enabled)
		SyntheticEnabledSet(AccountId, LiquidityPoolId, CurrencyId, bool),

		/// Max spread updated: (currency_id, spread)
		MaxSpreadUpdated(CurrencyId, Balance),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Set bid and ask spread of `currency_id` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = 10_000]
		pub fn set_spread(origin, #[compact] pool_id: LiquidityPoolId, currency_id: CurrencyId, #[compact] bid: Balance, #[compact] ask: Balance) {
			let who = ensure_signed(origin)?;
			Self::_set_spread(&who, pool_id, currency_id, bid, ask)?;
			Self::deposit_event(RawEvent::SpreadSet(who, pool_id, currency_id, bid, ask));
		}

		/// Set additional collateral ratio of `currency_id` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = 10_000]
		pub fn set_additional_collateral_ratio(origin, #[compact] pool_id: LiquidityPoolId, currency_id: CurrencyId, ratio: Option<Permill>) {
			let who = ensure_signed(origin)?;
			Self::_set_additional_collateral_ratio(&who, pool_id, currency_id, ratio)?;
			Self::deposit_event(RawEvent::AdditionalCollateralRatioSet(who, pool_id, currency_id, ratio));
		}

		/// Set minimum additional collateral ratio.
		///
		/// May only be called from `UpdateOrigin` or root.
		#[weight = 10_000]
		pub fn set_min_additional_collateral_ratio(origin, #[compact] ratio: Permill) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			MinAdditionalCollateralRatio::put(ratio);
			Self::deposit_event(RawEvent::MinAdditionalCollateralRatioSet(ratio));
		}

		/// Enable or disable synthetic of `currency_id` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = 10_000]
		pub fn set_synthetic_enabled(origin, #[compact] pool_id: LiquidityPoolId, currency_id: CurrencyId, enabled: bool) {
			let who = ensure_signed(origin)?;
			Self::_set_synthetic_enabled(&who, pool_id, currency_id, enabled)?;
			Self::deposit_event(RawEvent::SyntheticEnabledSet(who, pool_id, currency_id, enabled));
		}

		/// Set max spread of `currency_id`.
		///
		/// May only be called from `UpdateOrigin` or root.
		#[weight = 10_000]
		pub fn set_max_spread(origin, currency_id: CurrencyId, #[compact] max_spread: Balance) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			MaxSpread::insert(currency_id, max_spread);
			Self::deposit_event(RawEvent::MaxSpreadUpdated(currency_id, max_spread));
		}
	}
}

decl_error! {
	/// Errors for the synthetic liquidity pools module.
	pub enum Error for Module<T: Trait> {
		/// Caller doesn't have permission.
		NoPermission,

		/// Spread is higher than max allowed.
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

	/// Check if pool exists
	fn pool_exists(pool_id: LiquidityPoolId) -> bool {
		T::BaseLiquidityPools::pool_exists(pool_id)
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
	fn bid_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Balance> {
		Self::pool_currency_options(&pool_id, &currency_id).bid_spread
	}

	fn ask_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Balance> {
		Self::pool_currency_options(&pool_id, &currency_id).ask_spread
	}

	fn additional_collateral_ratio(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Permill {
		let min_ratio = Self::min_additional_collateral_ratio();
		Self::pool_currency_options(&pool_id, &currency_id)
			.additional_collateral_ratio
			.map_or(min_ratio, |ratio| ratio.max(min_ratio))
	}

	fn can_mint(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> bool {
		Self::pool_currency_options(&pool_id, &currency_id).synthetic_enabled
	}
}

// Private methods
impl<T: Trait> Module<T> {
	fn _set_spread(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		bid: Balance,
		ask: Balance,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		let max_spread = Self::max_spread(&currency_id);
		if !max_spread.is_zero() {
			ensure!(ask <= max_spread && bid <= max_spread, Error::<T>::SpreadTooHigh);
		}

		PoolCurrencyOptions::mutate(pool_id, currency_id, |o| {
			o.bid_spread = Some(bid);
			o.ask_spread = Some(ask);
		});

		Ok(())
	}

	fn _set_additional_collateral_ratio(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		ratio: Option<Permill>,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		PoolCurrencyOptions::mutate(pool_id, currency_id, |o| o.additional_collateral_ratio = ratio);
		Ok(())
	}

	fn _set_synthetic_enabled(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		enabled: bool,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		PoolCurrencyOptions::mutate(pool_id, currency_id, |o| o.synthetic_enabled = enabled);
		Ok(())
	}
}

impl<T: Trait> OnDisableLiquidityPool for Module<T> {
	fn on_disable(pool_id: LiquidityPoolId) {
		PoolCurrencyOptions::remove_prefix(&pool_id);
	}
}

impl<T: Trait> OnRemoveLiquidityPool for Module<T> {
	fn on_remove(pool_id: LiquidityPoolId) {
		PoolCurrencyOptions::remove_prefix(&pool_id);
	}
}
