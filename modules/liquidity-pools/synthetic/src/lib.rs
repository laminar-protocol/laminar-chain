#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::EnsureOrigin, weights::Weight};
use frame_system::ensure_signed;
use orml_utilities::with_transaction_result;
use primitives::{Balance, CurrencyId, LiquidityPoolId, Price};
use sp_runtime::{DispatchResult, ModuleId, Permill, RuntimeDebug};
use sp_std::prelude::*;
use traits::{LiquidityPools, OnDisableLiquidityPool, OnRemoveLiquidityPool, SyntheticProtocolLiquidityPools};

mod default_weight;
mod mock;
mod tests;

pub trait WeightInfo {
	fn set_spread() -> Weight;
	fn set_additional_collateral_ratio() -> Weight;
	fn set_min_additional_collateral_ratio() -> Weight;
	fn set_synthetic_enabled() -> Weight;
	fn set_max_spread() -> Weight;
}

use codec::{Decode, Encode};

/// Currency option in a pool of synthetic.
#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct SyntheticPoolCurrencyOption {
	/// Bid spread.
	///
	/// DEFAULT-NOTE: `None`, pool owner must set spread.
	pub bid_spread: Option<Price>,

	/// Ask spread.
	///
	/// DEFAULT-NOTE: `None`, pool owner must set spread.
	pub ask_spread: Option<Price>,

	/// Additional collateral ratio.
	///
	/// DEFAULT-NOTE: `None`. If not set or smaller than min additional swap rate, min value will be
	/// used instead.
	pub additional_collateral_ratio: Option<Permill>,

	/// Is a synthetic currency enabled to mint in the pool.
	///
	/// DEFAULT-NOTE: default not enabled.
	pub synthetic_enabled: bool,
}

pub const MODULE_ID: ModuleId = ModuleId(*b"lami/slp");

pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// The `LiquidityPools` implementation.
	type BaseLiquidityPools: LiquidityPools<Self::AccountId>;

	/// Required origin for updating protocol options.
	type UpdateOrigin: EnsureOrigin<Self::Origin>;

	/// Weight information for the extrinsics in this module.
	type WeightInfo: WeightInfo;
}

decl_storage! {
	trait Store for Module<T: Config> as SyntheticLiquidityPools {
		/// Currency options in a liquidity pool.
		pub PoolCurrencyOptions: double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) CurrencyId => SyntheticPoolCurrencyOption;

		/// Minimum additional collateral ratio.
		pub MinAdditionalCollateralRatio get(fn min_additional_collateral_ratio) config(): Permill;

		/// Maximum spread of a currency.
		pub MaxSpread get(fn max_spread): map hasher(twox_64_concat) CurrencyId => Option<Price>;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Config>::AccountId,
	{
		/// Spread set: \[who, pool_id, currency_id, bid, ask\]
		SpreadSet(AccountId, LiquidityPoolId, CurrencyId, Price, Price),

		/// Additional collateral ratio set: \[who, pool_id, currency_id, ratio\]
		AdditionalCollateralRatioSet(AccountId, LiquidityPoolId, CurrencyId, Option<Permill>),

		/// Min additional collateral ratio set: \[min_additional_collateral_ratio\]
		MinAdditionalCollateralRatioSet(Permill),

		/// Synthetic enabled set: \[who, pool_id, currency_id, is_enabled\]
		SyntheticEnabledSet(AccountId, LiquidityPoolId, CurrencyId, bool),

		/// Max spread updated: \[currency_id, spread\]
		MaxSpreadUpdated(CurrencyId, Price),
	}
);

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Set bid and ask spread of `currency_id` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::set_spread()]
		pub fn set_spread(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			currency_id: CurrencyId,
			#[compact] bid: Price,
			#[compact] ask: Price
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_set_spread(&who, pool_id, currency_id, bid, ask)?;
				Self::deposit_event(RawEvent::SpreadSet(who, pool_id, currency_id, bid, ask));
				Ok(())
			})?;
		}

		/// Set additional collateral ratio of `currency_id` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::set_additional_collateral_ratio()]
		pub fn set_additional_collateral_ratio(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			currency_id: CurrencyId,
			ratio: Option<Permill>
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_set_additional_collateral_ratio(&who, pool_id, currency_id, ratio)?;
				Self::deposit_event(RawEvent::AdditionalCollateralRatioSet(who, pool_id, currency_id, ratio));
				Ok(())
			})?;
		}

		/// Set minimum additional collateral ratio.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_min_additional_collateral_ratio()]
		pub fn set_min_additional_collateral_ratio(origin, #[compact] ratio: Permill) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				MinAdditionalCollateralRatio::put(ratio);
				Self::deposit_event(RawEvent::MinAdditionalCollateralRatioSet(ratio));
				Ok(())
			})?;
		}

		/// Enable or disable synthetic of `currency_id` in `pool_id`.
		///
		/// May only be called from the pool owner.
		#[weight = T::WeightInfo::set_synthetic_enabled()]
		pub fn set_synthetic_enabled(
			origin,
			#[compact] pool_id: LiquidityPoolId,
			currency_id: CurrencyId,
			enabled: bool
		) {
			with_transaction_result(|| {
				let who = ensure_signed(origin)?;
				Self::do_set_synthetic_enabled(&who, pool_id, currency_id, enabled)?;
				Self::deposit_event(RawEvent::SyntheticEnabledSet(who, pool_id, currency_id, enabled));
				Ok(())
			})?;
		}

		/// Set max spread of `currency_id`.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_max_spread()]
		pub fn set_max_spread(origin, currency_id: CurrencyId, #[compact] max_spread: Price) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				MaxSpread::insert(currency_id, max_spread);
				Self::deposit_event(RawEvent::MaxSpreadUpdated(currency_id, max_spread));
				Ok(())
			})?;
		}
	}
}

decl_error! {
	/// Errors for the synthetic liquidity pools module.
	pub enum Error for Module<T: Config> {
		/// Caller doesn't have permission.
		NoPermission,

		/// Spread is higher than max allowed.
		SpreadTooHigh,
	}
}

impl<T: Config> LiquidityPools<T::AccountId> for Module<T> {
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

impl<T: Config> SyntheticProtocolLiquidityPools<T::AccountId> for Module<T> {
	fn bid_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Price> {
		Self::pool_currency_options(pool_id, currency_id).bid_spread
	}

	fn ask_spread(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<Price> {
		Self::pool_currency_options(pool_id, currency_id).ask_spread
	}

	fn additional_collateral_ratio(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Permill {
		let min_ratio = Self::min_additional_collateral_ratio();
		Self::pool_currency_options(pool_id, currency_id)
			.additional_collateral_ratio
			.map_or(min_ratio, |ratio| ratio.max(min_ratio))
	}

	fn can_mint(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> bool {
		Self::pool_currency_options(pool_id, currency_id).synthetic_enabled
	}
}

// Storage getters.
impl<T: Config> Module<T> {
	/// `PoolCurrencyOptions` getter. Bid/ask spread is capped by max spread.
	pub fn pool_currency_options(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> SyntheticPoolCurrencyOption {
		let mut option = PoolCurrencyOptions::get(pool_id, currency_id);
		if let Some(max_spread) = Self::max_spread(currency_id) {
			option.bid_spread = option.bid_spread.map(|s| s.min(max_spread));
			option.ask_spread = option.ask_spread.map(|s| s.min(max_spread));
		}
		option
	}
}

// Dispatchable calls implementation
impl<T: Config> Module<T> {
	fn do_set_spread(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		bid: Price,
		ask: Price,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);

		if let Some(max_spread) = Self::max_spread(&currency_id) {
			ensure!(ask <= max_spread && bid <= max_spread, Error::<T>::SpreadTooHigh);
		}

		PoolCurrencyOptions::mutate(pool_id, currency_id, |o| {
			o.bid_spread = Some(bid);
			o.ask_spread = Some(ask);
		});

		Ok(())
	}

	fn do_set_additional_collateral_ratio(
		who: &T::AccountId,
		pool_id: LiquidityPoolId,
		currency_id: CurrencyId,
		ratio: Option<Permill>,
	) -> DispatchResult {
		ensure!(Self::is_owner(pool_id, who), Error::<T>::NoPermission);
		PoolCurrencyOptions::mutate(pool_id, currency_id, |o| o.additional_collateral_ratio = ratio);
		Ok(())
	}

	fn do_set_synthetic_enabled(
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

impl<T: Config> OnDisableLiquidityPool for Module<T> {
	fn on_disable(pool_id: LiquidityPoolId) {
		PoolCurrencyOptions::remove_prefix(&pool_id);
	}
}

impl<T: Config> OnRemoveLiquidityPool for Module<T> {
	fn on_remove(pool_id: LiquidityPoolId) {
		PoolCurrencyOptions::remove_prefix(&pool_id);
	}
}
