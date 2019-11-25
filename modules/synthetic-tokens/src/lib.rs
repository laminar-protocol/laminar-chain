#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_module, decl_storage, Parameter};
use sr_primitives::{
	traits::{MaybeSerializeDeserialize, Member, Zero},
	Permill,
};

use orml_traits::MultiCurrency;

type BalanceOf<T> = <<T as Trait>::Currency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::Balance;
type CurrencyIdOf<T> = <<T as Trait>::Currency as MultiCurrency<<T as frame_system::Trait>::AccountId>>::CurrencyId;

pub trait Trait: frame_system::Trait {
	type Currency: MultiCurrency<Self::AccountId>;
	type LiquidityPoolId: Parameter + Member + Copy + MaybeSerializeDeserialize;
}

#[derive(Encode, Decode)]
pub struct Position<T: Trait> {
	collateral_amount: BalanceOf<T>,
	minted_amount: BalanceOf<T>,
}

impl<T: Trait> Default for Position<T> {
	fn default() -> Self {
		Position {
			collateral_amount: Zero::zero(),
			minted_amount: Zero::zero(),
		}
	}
}

const EXTREME_RATIO_DEFAULT: Permill = Permill::from_percent(1); // TODO: set this
const LIQUIDATION_RATIO_DEFAULT: Permill = Permill::from_percent(5); // TODO: set this
const COLLATERAL_RATIO_DEFAULT: Permill = Permill::from_percent(10); // TODO: set this

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticTokens {
		ExtremeRatio get(extreme_ratio): map CurrencyIdOf<T> => Option<Permill>;
		LiquidationRatio get(liquidation_ratio): map CurrencyIdOf<T> => Option<Permill>;
		CollateralRatio get(collateral_ratio): map CurrencyIdOf<T> => Option<Permill>;
		Positions get(positions): map (T::LiquidityPoolId, CurrencyIdOf<T>) => Position<T>;
	}
}

decl_error! {
	pub enum Error {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

impl<T: Trait> Module<T> {
	pub fn add_position(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		collateral_amount: BalanceOf<T>,
		minted_amount: BalanceOf<T>,
	) {
		unimplemented!()
	}

	pub fn remove_position(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		collateral_amount: BalanceOf<T>,
		minted_amount: BalanceOf<T>,
	) {
		unimplemented!()
	}
}
