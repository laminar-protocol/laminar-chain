use super::utils::{dollars, set_ausd_balance};
use crate::{AccountId, BaseLiquidityPoolsForMargin, BaseLiquidityPoolsMarginInstance, Runtime};

use frame_system::{self as frame_system, RawOrigin};
use sp_runtime::DispatchError;
use sp_std::prelude::*;

use frame_benchmarking::account;
use orml_benchmarking::runtime_benchmarks_instance;

const SEED: u32 = 0;
const MAX_POOL_INDEX: u32 = 1000;
const MAX_DOLLARS: u32 = 1000;

fn new_pool(p: u32) -> Result<AccountId, DispatchError> {
	let owner: AccountId = account("owner", p, SEED);
	BaseLiquidityPoolsForMargin::create_pool(RawOrigin::Signed(owner.clone()).into())?;

	Ok(owner)
}

runtime_benchmarks_instance! {
	{ Runtime, base_liquidity_pools, BaseLiquidityPoolsMarginInstance }

	_ {
		let p in 1 .. MAX_POOL_INDEX => ();
		// make min as 11 dollars for the existential deposit
		let d in 11 .. MAX_DOLLARS => ();
	}

	create_pool {
		let p in ...;
		let owner: AccountId = account("owner", p, SEED);
	}: _(RawOrigin::Signed(owner))

	disable_pool {
		let p in ...;
		let owner = new_pool(p)?;
	}: _(RawOrigin::Signed(owner), 0)

	remove_pool {
		let p in ...;
		let owner = new_pool(p)?;
	}: _(RawOrigin::Signed(owner), 0)

	deposit_liquidity {
		let p in ...;
		let d in ...;

		let owner = new_pool(p)?;

		let balance = dollars(d);
		set_ausd_balance(&owner, balance + dollars(1u128))?;
	}: _(RawOrigin::Signed(owner), 0, balance)

	withdraw_liquidity {
		let p in ...;
		let d in ...;

		let owner = new_pool(p)?;

		let balance = dollars(d);
		set_ausd_balance(&owner, balance + dollars(1u128))?;

		BaseLiquidityPoolsForMargin::deposit_liquidity(RawOrigin::Signed(owner.clone()).into(), 0, balance)?;
	}: _(RawOrigin::Signed(owner), 0, balance - dollars(10u128)) // left 10 dollars for the existential deposit
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

	#[test]
	fn disable_pool() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_disable_pool());
		});
	}

	#[test]
	fn remove_pool() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_remove_pool());
		});
	}

	#[test]
	fn deposit_liquidity() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_deposit_liquidity());
		});
	}

	#[test]
	fn withdraw_liquidity() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_withdraw_liquidity());
		});
	}
}
