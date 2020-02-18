use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct SwapPeriod<Moment> {
	pub period: Moment,
	pub start: Moment,
}
