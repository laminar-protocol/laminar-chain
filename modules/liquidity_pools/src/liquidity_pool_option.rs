use codec::{Decode, Encode};
use sr_primitives::{Perbill, RuntimeDebug};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LiquidityPoolOption {
	pub bid_spread: Perbill,
	pub ask_spread: Perbill,
	pub additional_collateral_ratio: Option<Perbill>,
}
