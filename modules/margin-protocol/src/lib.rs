#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_error, decl_event, decl_module, decl_storage, traits::Currency};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system as system;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Currency: Currency<Self::AccountId>;
}

decl_storage! {
	trait Store for Module<T: Trait> as MarginProtocol {}
}

decl_event! {
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
	{
		Dummy(AccountId),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;
	}
}

impl<T: Trait> Module<T> {}
