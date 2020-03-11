#![cfg_attr(not(feature = "std"), no_std)]

use orml_utilities::{Fixed128, FixedU128};
use sp_arithmetic::traits::UniqueSaturatedInto;

/// Create a `Fixed128` from `FixU128`. Note it could be lossy.
pub fn fixed_128_from_fixed_u128(f: FixedU128) -> Fixed128 {
	let parts: i128 = f.deconstruct().unique_saturated_into();
	Fixed128::from_parts(parts)
}
