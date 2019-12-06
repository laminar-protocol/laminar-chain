use codec::{Decode, Encode};
use sp_runtime::{Permill, RuntimeDebug};

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Default)]
pub struct LiquidityPoolOption {
	pub bid_spread: Permill,
	pub ask_spread: Permill,
	pub additional_collateral_ratio: Option<Permill>,
}
