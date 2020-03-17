#![cfg_attr(not(feature = "std"), no_std)]

use orml_utilities::{Fixed128, FixedU128};
use sp_arithmetic::traits::{Saturating, UniqueSaturatedInto};

/// Create a `Fixed128` from `FixedU128`. Note it could be lossy.
pub fn fixed_128_from_fixed_u128(f: FixedU128) -> Fixed128 {
	let parts: i128 = f.deconstruct().unique_saturated_into();
	Fixed128::from_parts(parts)
}

pub fn fixed_128_mul_signum(f: Fixed128, signum: i128) -> Fixed128 {
	Fixed128::from_parts(f.deconstruct().saturating_mul(signum))
}

pub fn fixed_128_from_u128(u: u128) -> Fixed128 {
	Fixed128::from_parts(u.unique_saturated_into())
}

/// Create a `u128` from `Fixed128` by saturating.
pub fn u128_from_fixed_128(f: Fixed128) -> u128 {
	f.deconstruct().unique_saturated_into()
}
