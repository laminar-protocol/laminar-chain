#![cfg_attr(not(feature = "std"), no_std)]

use palette_support::{decl_event, decl_module, decl_storage, decl_error};
// FIXME: `pallet/palette-` prefix should be used for all pallet modules, but currently `palette_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use palette_system as system;

pub trait Trait: palette_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as palette_system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as SyntheticTokens {}
}

decl_event! {
	pub enum Event<T> where
		<T as palette_system::Trait>::AccountId,
	{
		Dummy(AccountId),
	}
}

decl_error! {
	pub enum Error {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;
	}
}

impl<T: Trait> Module<T> {}
