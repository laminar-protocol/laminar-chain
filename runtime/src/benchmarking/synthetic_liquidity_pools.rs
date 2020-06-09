use crate::{AccountId, BaseLiquidityPoolsForSynthetic, Runtime};

use frame_system::RawOrigin;
use sp_runtime::{DispatchError, Permill};
use sp_std::prelude::*;

use frame_benchmarking::account;
use orml_benchmarking::runtime_benchmarks;

use module_primitives::CurrencyId::*;

const SEED: u32 = 0;
const MAX_POOL_INDEX: u32 = 1000;
const MAX_SPREAD: u32 = 1000;
const MAX_ADDITIONAL_COLLATERAL_RATIO: u32 = 1000;

fn create_pool(p: u32) -> Result<AccountId, DispatchError> {
	let owner: AccountId = account("owner", p, SEED);
	BaseLiquidityPoolsForSynthetic::create_pool(RawOrigin::Signed(owner.clone()).into())?;

	Ok(owner)
}

runtime_benchmarks! {
	{ Runtime, synthetic_liquidity_pools }

	_ {
		let p in 1 .. MAX_POOL_INDEX => ();
		let s in 1 .. MAX_SPREAD => ();
		let r in 1 .. MAX_ADDITIONAL_COLLATERAL_RATIO => ();
	}

	set_spread {
		let p in ...;
		let s in ...;
		let owner = create_pool(p)?;
	}: _(RawOrigin::Signed(owner), 0, FEUR, s.into(), s.into())

	set_additional_collateral_ratio {
		let p in ...;
		let r in ...;
		let owner = create_pool(p)?;
	}: _(RawOrigin::Signed(owner), 0, FEUR, Some(Permill::from_inner(r)))

	set_min_additional_collateral_ratio {
		let r in ...;
	}: _(RawOrigin::Root, Permill::from_inner(r))

	set_synthetic_enabled {
		let p in ...;
		let owner = create_pool(p)?;
	}: _(RawOrigin::Signed(owner), 0, FEUR, true)

	set_max_spread {
		let s in ...;
	}: _(RawOrigin::Root, FEUR, s.into())
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;

	fn new_test_ext() -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap()
			.into()
	}

	#[test]
	fn set_spread() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_spread());
		});
	}

	#[test]
	fn set_additional_collateral_ratio() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_additional_collateral_ratio());
		});
	}

	#[test]
	fn set_min_additional_collateral_ratio() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_min_additional_collateral_ratio());
		});
	}

	#[test]
	fn set_synthetic_enabled() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_synthetic_enabled());
		});
	}

	#[test]
	fn set_max_spread() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_max_spread());
		});
	}
}
