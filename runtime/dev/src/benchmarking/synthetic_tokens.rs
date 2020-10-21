use crate::{Permill, Runtime};

use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use primitives::CurrencyId;
use sp_std::prelude::*;

runtime_benchmarks! {
	{ Runtime, synthetic_tokens }

	_ {}

	set_extreme_ratio {
	}: _(RawOrigin::Root, CurrencyId::FEUR, Permill::from_percent(1))

	set_liquidation_ratio {
	}: _(RawOrigin::Root, CurrencyId::FEUR, Permill::from_percent(1))

	set_collateral_ratio {
	}: _(RawOrigin::Root, CurrencyId::FEUR, Permill::from_percent(1))
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;

	fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		pallet_membership::GenesisConfig::<Runtime, pallet_membership::Instance3> {
			members: vec![AccountId::from([100u8; 32]), AccountId::from([101u8; 32])],
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}

	#[test]
	fn test_set_extreme_ratio() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_extreme_ratio());
		});
	}

	#[test]
	fn test_set_liquidation_ratio() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_liquidation_ratio());
		});
	}

	#[test]
	fn test_set_collateral_ratio() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_collateral_ratio());
		});
	}
}
