use orml_utilities::{FixedU128, FixedUnsignedNumber};
use sp_arithmetic::{traits::UniqueSaturatedInto, FixedI128, FixedPointNumber};

/// Create a `FixedI128` from `FixedU128`. Note it could be lossy.
pub fn fixed_i128_from_fixed_u128(f: FixedU128) -> FixedI128 {
	let parts: i128 = f.into_inner().unique_saturated_into();
	FixedI128::from_inner(parts)
}

pub fn fixed_i128_mul_signum(f: FixedI128, signum: i128) -> FixedI128 {
	FixedI128::from_inner(f.into_inner().saturating_mul(signum))
}

pub fn fixed_i128_from_u128(u: u128) -> FixedI128 {
	FixedI128::from_inner(u.unique_saturated_into())
}

/// Create a `u128` from `FixedI128` by saturating. Returns zero if `f` is negative.
pub fn u128_from_fixed_i128(f: FixedI128) -> u128 {
	if f.is_negative() {
		return 0u128;
	}

	f.into_inner().unique_saturated_into()
}
