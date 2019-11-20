use codec::{Decode, Encode};
use sr_primitives::{Perbill, RuntimeDebug};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq)]
pub struct LiquidityPool {
	pub bid_spread: Perbill,
	pub ask_spread: Perbill,
	pub additional_collateral_ratio: Option<Perbill>,
}

impl Default for LiquidityPool {
	fn default() -> LiquidityPool {
		LiquidityPool {
			bid_spread: Perbill::one(),
			ask_spread: Perbill::one(),
			additional_collateral_ratio: Some(Perbill::from_percent(110)),
		}
	}
}
