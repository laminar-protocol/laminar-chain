#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_module, decl_storage, Parameter};
use sr_primitives::{
	traits::{MaybeSerializeDeserialize, Member, Saturating, Zero},
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
	collateral: BalanceOf<T>,
	minted: BalanceOf<T>,
}

impl<T: Trait> Position<T> {
	fn new(collateral: BalanceOf<T>, minted: BalanceOf<T>) -> Self {
		Position { collateral, minted }
	}
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
		currency_id: CurrencyIdOf<T>,
		collateral: BalanceOf<T>,
		minted: BalanceOf<T>,
	) {
		<Positions<T>>::mutate((pool_id, currency_id), |p| {
			p.collateral = p.collateral.saturating_add(collateral);
			p.minted = p.minted.saturating_add(minted)
		});
	}

	pub fn remove_position(
		who: T::AccountId,
		pool_id: T::LiquidityPoolId,
		currency_id: CurrencyIdOf<T>,
		collateral: BalanceOf<T>,
		minted: BalanceOf<T>,
	) {
		<Positions<T>>::mutate((pool_id, currency_id), |p| {
			p.collateral = p.collateral.saturating_sub(collateral);
			p.minted = p.minted.saturating_sub(minted)
		});
	}
}
