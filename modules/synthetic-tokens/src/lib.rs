#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_module, decl_storage, Parameter};
use sr_primitives::{
	traits::{MaybeSerializeDeserialize, Member, Saturating, SimpleArithmetic, Zero, AccountIdConversion},
	Permill, ModuleId,
};

pub trait Trait: frame_system::Trait {
	type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize;
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type LiquidityPoolId: Parameter + Member + Copy + MaybeSerializeDeserialize;
}

#[derive(Encode, Decode)]
pub struct Position<T: Trait> {
	collateral: T::Balance,
	minted: T::Balance,
}

impl<T: Trait> Default for Position<T> {
	fn default() -> Self {
		Position {
			collateral: Zero::zero(),
			minted: Zero::zero(),
		}
	}
}

const _EXTREME_RATIO_DEFAULT: Permill = Permill::from_percent(1); // TODO: set this
const _LIQUIDATION_RATIO_DEFAULT: Permill = Permill::from_percent(5); // TODO: set this
const _COLLATERAL_RATIO_DEFAULT: Permill = Permill::from_percent(10); // TODO: set this

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

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
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
		minted: T::Balance,
	) {
		<Positions<T>>::mutate((pool_id, currency_id), |p| {
			p.collateral = p.collateral.saturating_add(collateral);
			p.minted = p.minted.saturating_add(minted)
		});
	}

	pub fn remove_position(
		pool_id: T::LiquidityPoolId,
		currency_id: T::CurrencyId,
		collateral: T::Balance,
		minted: T::Balance,
	) {
		<Positions<T>>::mutate((pool_id, currency_id), |p| {
			p.collateral = p.collateral.saturating_sub(collateral);
			p.minted = p.minted.saturating_sub(minted)
		});
	}
}
