#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;
mod timestamped_value;

use sr_primitives::traits::Member;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, traits::Time, Parameter};
use system::{ensure_root, ensure_signed};
use timestamped_value::TimestampedValue;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Key: Parameter + Member + Copy;
	type Value: Parameter + Member + Copy;
	type Time: Time;
}

type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

decl_storage! {
	trait Store for Module<T: Trait> as Flow {
		pub Operators get(operators): Vec<T::AccountId>;
		pub RawValues get(raw_values): map (T::AccountId, T::Key) => Option<TimestampedValue<T::Value, MomentOf<T>>>;
		pub HasUpdate get(has_update): map T::Key => bool;
		pub Values get(values): map T::Key => Option<T::Value>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn add_operator(origin, operator: T::AccountId) -> Result {
			ensure_root(origin)?;
			Self::add_operator_internal(operator)
		}

		pub fn remove_operator(origin, operator: T::AccountId) -> Result {
			ensure_root(origin)?;
			Self::remove_operator_internal(operator)
		}

		pub fn feed_data(origin, key: T::Key, value: T::Value) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(Self::is_operator(&who), "Only operators can feed data");
			Self::feed_data_internal(who, key, value)
		}
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::Key,
		<T as Trait>::Value,
	{
		OperatorAdded(AccountId),
		OperatorRemoved(AccountId),
		NewFeedData(AccountId, Key, Value),
	}
);

impl<T: Trait> Module<T> {
	pub fn read_raw_values(key: &T::Key) -> Vec<TimestampedValue<T::Value, MomentOf<T>>> {
		<Operators<T>>::get()
			.iter()
			.filter_map(|x| <RawValues<T>>::get((x, *key)))
			.collect()
	}
}

impl<T: Trait> Module<T> {
	fn is_operator(who: &T::AccountId) -> bool {
		<Operators<T>>::get().contains(who)
	}

	fn add_operator_internal(operator: T::AccountId) -> Result {
		let mut operatros = <Operators<T>>::get();
		operatros.append(&mut vec![operator.clone()]);
		<Operators<T>>::put(operatros);

		Self::deposit_event(RawEvent::OperatorAdded(operator));
		Ok(())
	}

	fn remove_operator_internal(operator: T::AccountId) -> Result {
		let mut operatros = <Operators<T>>::get();
		match operatros.iter().position(|x| *x == operator) {
			Some(index) => {
				operatros.remove(index);
				<Operators<T>>::put(operatros);

				Self::deposit_event(RawEvent::OperatorRemoved(operator));
				Ok(())
			}
			None => panic!("Operator doesn't exists"),
		}
	}

	fn feed_data_internal(who: T::AccountId, key: T::Key, value: T::Value) -> Result {
		let timestamp = TimestampedValue {
			value,
			timestamp: T::Time::now(),
		};
		<RawValues<T>>::insert((who.clone(), key), timestamp);
		<HasUpdate<T>>::insert(key, true);

		Self::deposit_event(RawEvent::NewFeedData(who, key, value));
		Ok(())
	}
}
