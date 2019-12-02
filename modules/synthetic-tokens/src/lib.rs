#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, Parameter, StorageMap};
use frame_system::{self as system, ensure_root};
use sr_primitives::{
	traits::{AccountIdConversion, MaybeSerializeDeserialize, Member, Saturating, SimpleArithmetic, Zero},
	ModuleId, Permill,
};

use orml_utilities::FixedU128;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize;
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type LiquidityPoolId: Parameter + Member + Copy + MaybeSerializeDeserialize;
}

#[derive(Encode, Decode)]
pub struct Position<T: Trait> {
	collateral: T::Balance,
	synthetic: T::Balance,
}

impl<T: Trait> Default for Position<T> {
	fn default() -> Self {
		Position {
			collateral: Zero::zero(),
			synthetic: Zero::zero(),
		}
	}
}

const EXTREME_RATIO_DEFAULT: Permill = Permill::from_percent(1); // TODO: set this
const LIQUIDATION_RATIO_DEFAULT: Permill = Permill::from_percent(5); // TODO: set this
const COLLATERAL_RATIO_DEFAULT: Permill = Permill::from_percent(10); // TODO: set this

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticTokens {
		ExtremeRatio get(extreme_ratio): map T::CurrencyId => Option<Permill>;
		LiquidationRatio get(liquidation_ratio): map T::CurrencyId => Option<Permill>;
		CollateralRatio get(collateral_ratio): map T::CurrencyId => Option<Permill>;
		Positions get(positions): map (T::LiquidityPoolId, T::CurrencyId) => Position<T>;
	}
}

decl_error! {
	pub enum Error {}
}

decl_event! {
	pub enum Event<T> where
		CurrencyId = <T as Trait>::CurrencyId,
	{
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
		fn deposit_event() = default;

		pub fn set_extreme_ratio(origin, currency_id: T::CurrencyId, ratio: Permill) {
			ensure_root(origin)?;
			<ExtremeRatio<T>>::insert(currency_id, ratio);

			Self::deposit_event(RawEvent::ExtremeRatioUpdated(currency_id, ratio));
		}

		pub fn set_liquidation_ratio(origin, currency_id: T::CurrencyId, ratio: Permill) {
			ensure_root(origin)?;
			<LiquidationRatio<T>>::insert(currency_id, ratio);

			Self::deposit_event(RawEvent::LiquidationRatioUpdated(currency_id, ratio));
		}

		pub fn set_collateral_ratio(origin, currency_id: T::CurrencyId, ratio: Permill) {
			ensure_root(origin)?;
			<CollateralRatio<T>>::insert(currency_id, ratio);

			Self::deposit_event(RawEvent::CollateralRatioUpdated(currency_id, ratio));
		}
	}
}

/// The module id.
///
/// Note that module id is used to generate module account id for locking balances purpose.
/// DO NOT change this in runtime upgrade without migration.
const MODULE_ID: ModuleId = ModuleId(*b"FLOWTKNS");

impl<T: Trait> Module<T> {
	pub fn account_id() -> T::AccountId {
		MODULE_ID.into_account()
	}

	pub fn add_position(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		collateral: T::Balance,
		synthetic: T::Balance,
	) {
		<Positions<T>>::mutate((pool_id, currency_id), |p| {
			p.collateral = p.collateral.saturating_add(collateral);
			p.synthetic = p.synthetic.saturating_add(synthetic)
		});
	}

	pub fn remove_position(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		collateral: T::Balance,
		synthetic: T::Balance,
	) {
		<Positions<T>>::mutate((pool_id, currency_id), |p| {
			p.collateral = p.collateral.saturating_sub(collateral);
			p.synthetic = p.synthetic.saturating_sub(synthetic)
		});
	}

	/// Get position under `pool_id` and `currency_id`. Returns `(collateral_amount, synthetic_amount)`.
	pub fn get_position(pool_id: T::LiquidityPoolId, currency_id: T::CurrencyId) -> (T::Balance, T::Balance) {
		let Position { collateral, synthetic } = <Positions<T>>::get(&(pool_id, currency_id));
		(collateral, synthetic)
	}

	/// Calculate incentive ratio.
	///
	/// If `ratio < extreme_ratio`, return `1`; if `ratio >= liquidation_ratio`, return `0`; Otherwise return
	/// `(liquidation_ratio - ratio) / (liquidation_ratio - extreme_ratio)`.
	pub fn incentive_ratio(currency_id: T::CurrencyId, current_ratio: FixedU128) -> FixedU128 {
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
			.checked_sub(&liquidation_to_extreme_gap)
			.expect("liquidation_ratio > extreme_ratio; qed")
	}
}

impl<T: Trait> Module<T> {
	pub fn liquidation_ratio_or_default(currency_id: T::CurrencyId) -> Permill {
		Self::liquidation_ratio(currency_id).unwrap_or(LIQUIDATION_RATIO_DEFAULT)
	}

	pub fn extreme_ratio_or_default(currency_id: T::CurrencyId) -> Permill {
		Self::extreme_ratio(currency_id).unwrap_or(EXTREME_RATIO_DEFAULT)
	}

	pub fn collateral_ratio_or_default(currency_id: T::CurrencyId) -> Permill {
		Self::collateral_ratio(currency_id).unwrap_or(COLLATERAL_RATIO_DEFAULT)
	}
}
