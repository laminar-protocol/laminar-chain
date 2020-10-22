use super::utils::{dollars, set_ausd_balance, set_balance};
use crate::{AccountId, BaseLiquidityPoolsForMargin, BaseLiquidityPoolsMarginInstance, CurrencyId, Runtime};

use frame_benchmarking::account;
use frame_system::{self as frame_system, RawOrigin};
use orml_benchmarking::runtime_benchmarks_instance;
use primitives::IdentityInfo;
use sp_runtime::DispatchError;
use sp_std::prelude::*;

const SEED: u32 = 0;

fn new_pool() -> Result<AccountId, DispatchError> {
	let owner: AccountId = account("owner", 0, SEED);
	BaseLiquidityPoolsForMargin::create_pool(RawOrigin::Signed(owner.clone()).into())?;

	Ok(owner)
}

runtime_benchmarks_instance! {
	{ Runtime, base_liquidity_pools, BaseLiquidityPoolsMarginInstance }

	_ {}

	create_pool {
		let owner: AccountId = account("owner", 0, SEED);
	}: _(RawOrigin::Signed(owner))

	disable_pool {
		let owner = new_pool()?;
	}: _(RawOrigin::Signed(owner), 0)

	remove_pool {
		let owner = new_pool()?;
	}: _(RawOrigin::Signed(owner), 0)

	deposit_liquidity {
		let owner = new_pool()?;

		let balance = dollars(100u128);
		set_ausd_balance(&owner, balance + dollars(1u128))?;
	}: _(RawOrigin::Signed(owner), 0, balance)

	withdraw_liquidity {
		let owner = new_pool()?;

		let balance = dollars(100u128);
		set_ausd_balance(&owner, balance + dollars(1u128))?;

		BaseLiquidityPoolsForMargin::deposit_liquidity(RawOrigin::Signed(owner.clone()).into(), 0, balance)?;
	}: _(RawOrigin::Signed(owner), 0, balance - dollars(10u128)) // left 10 dollars for the existential deposit

	set_identity {
		let owner = new_pool()?;
		let identity = IdentityInfo {
			legal_name: "laminar".as_bytes().to_vec(),
			display_name: vec![],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};

		let balance = dollars(10000u128);
		set_balance(CurrencyId::LAMI, &owner, balance + dollars(1u128))?;
	}: _(RawOrigin::Signed(owner), 0, identity)

	verify_identity {
		let owner = new_pool()?;
		let identity = IdentityInfo {
			legal_name: "laminar".as_bytes().to_vec(),
			display_name: vec![],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};

		let balance = dollars(10000u128);
		set_balance(CurrencyId::LAMI, &owner, balance + dollars(1u128))?;

		BaseLiquidityPoolsForMargin::set_identity(RawOrigin::Signed(owner.clone()).into(), 0, identity)?;
	}: _(RawOrigin::Root, 0)

	clear_identity {
		let owner = new_pool()?;
		let identity = IdentityInfo {
			legal_name: "laminar".as_bytes().to_vec(),
			display_name: vec![],
			web: "https://laminar.one".as_bytes().to_vec(),
			email: vec![],
			image_url: vec![],
		};

		let balance = dollars(10000u128);
		set_balance(CurrencyId::LAMI, &owner, balance + dollars(1u128))?;

		BaseLiquidityPoolsForMargin::set_identity(RawOrigin::Signed(owner.clone()).into(), 0, identity)?;
	}: _(RawOrigin::Signed(owner), 0)

	transfer_liquidity_pool {
		let owner = new_pool()?;
		let to: AccountId = account("to", 0, SEED);
	}: _(RawOrigin::Signed(owner), 0, to)
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

	#[test]
	fn set_identity() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_identity());
		});
	}

	#[test]
	fn verify_identity() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_verify_identity());
		});
	}

	#[test]
	fn clear_identity() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_clear_identity());
		});
	}

	#[test]
	fn transfer_liquidity_pool() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_transfer_liquidity_pool());
		});
	}
}
