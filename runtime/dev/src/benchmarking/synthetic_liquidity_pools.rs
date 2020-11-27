use crate::{AccountId, BaseLiquidityPoolsForSynthetic, Runtime};

use frame_system::RawOrigin;
use sp_runtime::{DispatchError, Permill};
use sp_std::prelude::*;

use frame_benchmarking::account;
use orml_benchmarking::runtime_benchmarks;

use primitives::{CurrencyId::*, Price};

const SEED: u32 = 0;

fn create_pool() -> Result<AccountId, DispatchError> {
	let owner: AccountId = account("owner", 0, SEED);
	BaseLiquidityPoolsForSynthetic::create_pool(RawOrigin::Signed(owner.clone()).into())?;

	Ok(owner)
}

runtime_benchmarks! {
	{ Runtime, synthetic_liquidity_pools }

	_ {}

	set_spread {
		let owner = create_pool()?;
	}: _(RawOrigin::Signed(owner), 0, FEUR, Price::from_inner(10u128), Price::from_inner(10u128))

	set_additional_collateral_ratio {
		let owner = create_pool()?;
	}: _(RawOrigin::Signed(owner), 0, FEUR, Some(Permill::from_parts(10)))

	set_min_additional_collateral_ratio {
	}: _(RawOrigin::Root, Permill::from_parts(10))

	set_synthetic_enabled {
		let owner = create_pool()?;
	}: _(RawOrigin::Signed(owner), 0, FEUR, true)

	set_max_spread {
	}: _(RawOrigin::Root, FEUR, Price::from_inner(10u128))
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
