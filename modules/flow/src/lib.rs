#![cfg_attr(not(feature = "std"), no_std)]

use palette_support::{decl_event, decl_module, decl_storage, traits::Currency};
// FIXME: `pallet/palette-` prefix should be used for all pallet modules, but currently `palette_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use palette_system as system;

pub trait Trait: palette_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Currency: Currency<Self::AccountId>;
}

type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;

decl_storage! {
	trait Store for Module<T: Trait> as Flow {

	}
}

decl_event!(
	pub enum Event<T> where
		<T as palette_system::Trait>::AccountId,
		Balance = BalanceOf<T>,
	{
		Dummy(AccountId, Balance),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

	}
}

impl<T: Trait> Module<T> {}
