use codec::{Decode, Encode};
use sp_runtime::{Perbill, RuntimeDebug};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LiquidityPoolOption {
	pub bid_spread: Perbill,
	pub ask_spread: Perbill,
	pub additional_collateral_ratio: Option<Perbill>,
}
