#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	traits::{EnsureOrigin, Get},
	weights::Weight,
};
use laminar_primitives::{Balance, CurrencyId, LiquidityPoolId};
use module_traits::BaseLiquidityPoolManager;
use orml_utilities::with_transaction_result;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedDiv, CheckedSub, Zero},
	DispatchResult, FixedPointNumber, FixedU128, ModuleId, Permill, RuntimeDebug,
};
use sp_std::prelude::Vec;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

mod default_weight;
mod mock;
mod tests;

pub trait WeightInfo {
	fn set_extreme_ratio() -> Weight;
	fn set_liquidation_ratio() -> Weight;
	fn set_collateral_ratio() -> Weight;
}

pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;

	/// The default extreme liquidation ratio.
	type DefaultExtremeRatio: Get<Permill>;

	/// The default liquidation ratio.
	type DefaultLiquidationRatio: Get<Permill>;

	/// The default collateral ratio.
	type DefaultCollateralRatio: Get<Permill>;

	/// Synthetic currency IDs.
	type SyntheticCurrencyIds: Get<Vec<CurrencyId>>;

	/// Required origin for updating protocol options.
	type UpdateOrigin: EnsureOrigin<Self::Origin>;

	/// Weight information for the extrinsics in this module.
	type WeightInfo: WeightInfo;
}

/// Synthetic token position.
#[derive(Encode, Decode, Eq, PartialEq, RuntimeDebug, Default)]
pub struct Position {
	/// Collateral amount.
	collateral: Balance,

	/// Synthetic amount.
	synthetic: Balance,
}

/// Synthetic token ratio options.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, PartialEq, Default)]
pub struct SyntheticTokensRatio {
	/// Extreme liquidation ratio.
	pub extreme: Option<Permill>,

	/// Liquidation ratio.
	pub liquidation: Option<Permill>,

	/// Collateral ratio.
	pub collateral: Option<Permill>,
}

decl_storage! {
	trait Store for Module<T: Config> as SyntheticTokens {
		/// Ratios for each currency.
		Ratios get(fn ratios) config(): map hasher(twox_64_concat) CurrencyId => SyntheticTokensRatio;

		/// Positions of a currency in a pool
		Positions get(fn positions): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) CurrencyId => Position;
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {}
}

decl_event! {
	pub enum Event {
		/// Extreme ratio updated: \[currency_id, ratio\]
		ExtremeRatioUpdated(CurrencyId, Permill),

		/// Liquidation ratio updated: \[currency_id, ratio\]
		LiquidationRatioUpdated(CurrencyId, Permill),

		/// Collateral ratio updated: \[currency_id, ratio\]
		CollateralRatioUpdated(CurrencyId, Permill),
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const DefaultExtremeRatio: Permill = T::DefaultExtremeRatio::get();
		const DefaultLiquidationRatio: Permill = T::DefaultLiquidationRatio::get();
		const DefaultCollateralRatio: Permill = T::DefaultCollateralRatio::get();
		const SyntheticCurrencyIds: Vec<CurrencyId> = T::SyntheticCurrencyIds::get();

		/// Set extreme liquidation ratio.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_extreme_ratio()]
		pub fn set_extreme_ratio(origin, currency_id: CurrencyId, #[compact] ratio: Permill) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				Ratios::mutate(currency_id, |r| r.extreme = Some(ratio));
				Self::deposit_event(Event::ExtremeRatioUpdated(currency_id, ratio));
				Ok(())
			})?;
		}

		/// Set liquidation ratio.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_liquidation_ratio()]
		pub fn set_liquidation_ratio(origin, currency_id: CurrencyId, #[compact] ratio: Permill) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				Ratios::mutate(currency_id, |r| r.liquidation = Some(ratio));
				Self::deposit_event(Event::LiquidationRatioUpdated(currency_id, ratio));
				Ok(())
			})?;
		}

		/// Set collateral ratio.
		///
		/// May only be called from `UpdateOrigin`.
		#[weight = T::WeightInfo::set_collateral_ratio()]
		pub fn set_collateral_ratio(origin, currency_id: CurrencyId, #[compact] ratio: Permill) {
			with_transaction_result(|| {
				T::UpdateOrigin::ensure_origin(origin)?;
				Ratios::mutate(currency_id, |r| r.collateral = Some(ratio));
				Self::deposit_event(Event::CollateralRatioUpdated(currency_id, ratio));
				Ok(())
			})?;
		}
	}
}

/// The module id.
///
/// Note that module id is used to generate module account id for locking balances purpose.
/// DO NOT change this in runtime upgrade without migration.
const MODULE_ID: ModuleId = ModuleId(*b"lami/stk");

impl<T: Config> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn add_position(pool_id: LiquidityPoolId, currency_id: CurrencyId, collateral: Balance, synthetic: Balance) {
		Positions::mutate(&pool_id, currency_id, |p| {
			p.collateral = p.collateral.saturating_add(collateral);
			p.synthetic = p.synthetic.saturating_add(synthetic)
		});
	}

	pub fn remove_position(pool_id: LiquidityPoolId, currency_id: CurrencyId, collateral: Balance, synthetic: Balance) {
		Positions::mutate(&pool_id, currency_id, |p| {
			p.collateral = p.collateral.saturating_sub(collateral);
			p.synthetic = p.synthetic.saturating_sub(synthetic)
		});
	}

	/// Get position under `pool_id` and `currency_id`. Returns `(collateral_amount,
	/// synthetic_amount)`.
	pub fn get_position(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> (Balance, Balance) {
		let Position { collateral, synthetic } = Positions::get(&pool_id, currency_id);
		(collateral, synthetic)
	}

	/// Calculate incentive ratio.
	///
	/// If `ratio < extreme_ratio`, return `1`; if `ratio >= liquidation_ratio`, return `0`;
	/// Otherwise return `(liquidation_ratio - ratio) / (liquidation_ratio - extreme_ratio)`.
	pub fn incentive_ratio(currency_id: CurrencyId, current_ratio: FixedU128) -> FixedU128 {
		let one = FixedU128::one();
		if current_ratio < one {
			return FixedU128::from_inner(0);
		}

		let ratio = current_ratio
			.checked_sub(&one)
			.expect("ensured current_ratio > one_percent; qed");
		let liquidation_ratio = Self::liquidation_ratio_or_default(currency_id).into();
		if ratio >= liquidation_ratio {
			return FixedU128::from_inner(0);
		}
		let extreme_ratio = Self::extreme_ratio_or_default(currency_id).into();
		if ratio <= extreme_ratio {
			return one;
		}

		let ratio_to_liquidation_ratio_gap = liquidation_ratio
			.checked_sub(&ratio)
			.expect("ratio < liquidation_ratio; qed");
		let liquidation_to_extreme_gap = liquidation_ratio
			.checked_sub(&extreme_ratio)
			.expect("liquidation_ratio > extreme_ratio; qed");

		// ratio_to_liquidation_ratio_gap / liquidation_to_extreme_gap
		ratio_to_liquidation_ratio_gap
			.checked_div(&liquidation_to_extreme_gap)
			.expect("liquidation_ratio > extreme_ratio; qed")
	}
}

impl<T: Config> Module<T> {
	pub fn liquidation_ratio_or_default(currency_id: CurrencyId) -> Permill {
		Self::ratios(currency_id)
			.liquidation
			.unwrap_or_else(T::DefaultLiquidationRatio::get)
	}

	pub fn extreme_ratio_or_default(currency_id: CurrencyId) -> Permill {
		Self::ratios(currency_id)
			.extreme
			.unwrap_or_else(T::DefaultExtremeRatio::get)
	}

	pub fn collateral_ratio_or_default(currency_id: CurrencyId) -> Permill {
		Self::ratios(currency_id)
			.collateral
			.unwrap_or_else(T::DefaultCollateralRatio::get)
	}
}

impl<T: Config> BaseLiquidityPoolManager<LiquidityPoolId, Balance> for Module<T> {
	fn can_remove(pool_id: LiquidityPoolId) -> bool {
		T::SyntheticCurrencyIds::get()
			.iter()
			.map(|currency_id| -> (Balance, Balance) { Self::get_position(pool_id, *currency_id) })
			.all(|x| x.1.is_zero())
	}

	fn ensure_can_withdraw(_pool: LiquidityPoolId, _amount: Balance) -> DispatchResult {
		Ok(())
	}
}
