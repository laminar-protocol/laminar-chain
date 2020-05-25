use super::utils::dollars;
use crate::{AccountId, BlockNumber, Runtime};

use frame_system::{self as frame_system, RawOrigin};
use sp_runtime::{Fixed128, Permill};
use sp_std::prelude::*;

use frame_benchmarking::account;
use orml_benchmarking::runtime_benchmarks_instance;

use module_primitives::*;
use base_liquidity_pools::Instance;

type BaseLiquidityPools = base_liquidity_pools::Module<Runtime>;

const SEED: u32 = 0;
const MAX_POOL_INDEX: u32 = 1000;

fn new_pool(p: u32) -> AccountId {
	let caller: AccountId = account("caller", p, SEED);
	assert_ok!(BaseLiquidityPools::create_pool(RawOrigin::Signed(caller.clone()).into()));

	caller
}

runtime_benchmarks_instance! {
	{ Runtime, base_liquidity_pools }

	_ {
		let p in 1 .. MAX_POOL_INDEX => ();
	}

	create_pool {
		let owner: AccountId = account("owner", p, SEED);
	}: _(RawOrigin::Signed(owner.clone))
	verify {
		assert_eq!(BaseLiquidityPools::owners(0), Some(owner, 0))
	}
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
	fn create_pool() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_create_pool());
		});
	}
}
