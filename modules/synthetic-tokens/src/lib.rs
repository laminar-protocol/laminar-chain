#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use palette_support::{decl_error, decl_module, decl_storage};
use sr_primitives::{traits::Zero, Permill};

use orml_traits::MultiCurrency;

use module_primitives::{CurrencyId, LiquidityPoolId};

type BalanceOf<T> = <<T as Trait>::Currency as MultiCurrency<<T as palette_system::Trait>::AccountId>>::Balance;

pub trait Trait: palette_system::Trait {
	type Currency: MultiCurrency<Self::AccountId>;
}

#[derive(Encode, Decode)]
pub struct Position<T: Trait> {
	collateral: BalanceOf<T>,
	minted: BalanceOf<T>,
}

impl<T: Trait> Default for Position<T> {
	fn default() -> Self {
		Position {
			collateral: Zero::zero(),
			minted: Zero::zero(),
		}
	}
}

const EXTREME_RATIO_DEFAULT: Permill = Permill::from_percent(1); // TODO: set this
const LIQUIDATION_RATIO_DEFAULT: Permill = Permill::from_percent(5); // TODO: set this
const COLLATERAL_RATIO_DEFAULT: Permill = Permill::from_percent(10); // TODO: set this

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticTokens {
		ExtremeRatio get(extreme_ratio): map CurrencyId => Option<Permill>;
		LiquidationRatio get(liquidation_ratio): map CurrencyId => Option<Permill>;
		CollateralRatio get(collateral_ratio): map CurrencyId => Option<Permill>;
		Positions get(positions): map (LiquidityPoolId, CurrencyId) => Position<T>;
	}
}

decl_error! {
	pub enum Error {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

impl<T: Trait> Module<T> {
	pub fn add_position(who: T::AccountId, pool_id: LiquidityPoolId, collateral: BalanceOf<T>, minted: BalanceOf<T>) {
		unimplemented!()
	}

	pub fn remove_position(
		who: T::AccountId,
		pool_id: LiquidityPoolId,
		collateral: BalanceOf<T>,
		minted: BalanceOf<T>,
	) {
		unimplemented!()
	}
}
