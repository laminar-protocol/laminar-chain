use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct TradingPair {}
