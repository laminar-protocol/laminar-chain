#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	traits::{EnsureOrigin, Get},
};
use frame_system::{self as system, ensure_root};
use module_primitives::{Balance, CurrencyId, LiquidityPoolId};
use module_traits::BaseLiquidityPoolManager;
use orml_utilities::FixedU128;
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	DispatchResult, ModuleId, Permill, RuntimeDebug,
};
use sp_std::prelude::Vec;

mod mock;
mod tests;

pub trait Trait: frame_system::Trait {
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
	type DefaultExtremeRatio: Get<Permill>;
	type DefaultLiquidationRatio: Get<Permill>;
	type DefaultCollateralRatio: Get<Permill>;
	type SyntheticCurrencyIds: Get<Vec<CurrencyId>>;
	type UpdateOrigin: EnsureOrigin<Self::Origin>;
}

#[derive(Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct Position {
	collateral: Balance,
	synthetic: Balance,
}

impl Default for Position {
	fn default() -> Self {
		Position {
			collateral: Zero::zero(),
			synthetic: Zero::zero(),
		}
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticTokens {
		ExtremeRatio get(fn extreme_ratio): map hasher(twox_64_concat) CurrencyId => Option<Permill>;
		LiquidationRatio get(fn liquidation_ratio): map hasher(twox_64_concat) CurrencyId => Option<Permill>;
		CollateralRatio get(fn collateral_ratio): map hasher(twox_64_concat) CurrencyId => Option<Permill>;
		Positions get(fn positions): double_map hasher(twox_64_concat) LiquidityPoolId, hasher(twox_64_concat) CurrencyId => Position;
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {}
}

decl_event! {
	pub enum Event {
		/// Extreme ratio updated. (currency_id, ratio)
		ExtremeRatioUpdated(CurrencyId, Permill),
		/// Liquidation ratio updated. (currency_id, ratio)
		LiquidationRatioUpdated(CurrencyId, Permill),
		/// Collateral ratio updated. (currency_id, ratio)
		CollateralRatioUpdated(CurrencyId, Permill),
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const DefaultExtremeRatio: Permill = T::DefaultExtremeRatio::get();
		const DefaultLiquidationRatio: Permill = T::DefaultLiquidationRatio::get();
		const DefaultCollateralRatio: Permill = T::DefaultCollateralRatio::get();
		const SyntheticCurrencyIds: Vec<CurrencyId> = T::SyntheticCurrencyIds::get();

		#[weight = 10_000]
		pub fn set_extreme_ratio(origin, currency_id: CurrencyId, #[compact] ratio: Permill) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			ExtremeRatio::insert(currency_id, ratio);

			Self::deposit_event(Event::ExtremeRatioUpdated(currency_id, ratio));
		}

		#[weight = 10_000]
		pub fn set_liquidation_ratio(origin, currency_id: CurrencyId, #[compact] ratio: Permill) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			LiquidationRatio::insert(currency_id, ratio);

			Self::deposit_event(Event::LiquidationRatioUpdated(currency_id, ratio));
		}

		#[weight = 10_000]
		pub fn set_collateral_ratio(origin, currency_id: CurrencyId, #[compact] ratio: Permill) {
			T::UpdateOrigin::try_origin(origin)
				.map(|_| ())
				.or_else(ensure_root)?;
			CollateralRatio::insert(currency_id, ratio);

			Self::deposit_event(Event::CollateralRatioUpdated(currency_id, ratio));
		}
	}
}

/// The module id.
///
/// Note that module id is used to generate module account id for locking balances purpose.
/// DO NOT change this in runtime upgrade without migration.
const MODULE_ID: ModuleId = ModuleId(*b"lami/stk");

impl<T: Trait> Module<T> {
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

	/// Get position under `pool_id` and `currency_id`. Returns `(collateral_amount, synthetic_amount)`.
	pub fn get_position(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> (Balance, Balance) {
		let Position { collateral, synthetic } = Positions::get(&pool_id, currency_id);
		(collateral, synthetic)
	}

	/// Calculate incentive ratio.
	///
	/// If `ratio < extreme_ratio`, return `1`; if `ratio >= liquidation_ratio`, return `0`; Otherwise return
	/// `(liquidation_ratio - ratio) / (liquidation_ratio - extreme_ratio)`.
	pub fn incentive_ratio(currency_id: CurrencyId, current_ratio: FixedU128) -> FixedU128 {
		let one = FixedU128::from_rational(1, 1);
		if current_ratio < one {
			return FixedU128::from_parts(0);
		}

		let ratio = current_ratio
			.checked_sub(&one)
			.expect("ensured current_ratio > one_percent; qed");
		let liquidation_ratio = Self::liquidation_ratio_or_default(currency_id).into();
		if ratio >= liquidation_ratio {
			return FixedU128::from_parts(0);
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

impl<T: Trait> Module<T> {
	pub fn liquidation_ratio_or_default(currency_id: CurrencyId) -> Permill {
		Self::liquidation_ratio(currency_id).unwrap_or(T::DefaultLiquidationRatio::get())
	}

	pub fn extreme_ratio_or_default(currency_id: CurrencyId) -> Permill {
		Self::extreme_ratio(currency_id).unwrap_or(T::DefaultExtremeRatio::get())
	}

	pub fn collateral_ratio_or_default(currency_id: CurrencyId) -> Permill {
		Self::collateral_ratio(currency_id).unwrap_or(T::DefaultCollateralRatio::get())
	}
}

impl<T: Trait> BaseLiquidityPoolManager<LiquidityPoolId, Balance> for Module<T> {
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
