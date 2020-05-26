use super::utils::dollars;
use crate::{AccountId, BaseLiquidityPoolsForMargin, BlockNumber, MarginLiquidityPools, MarginProtocol, Runtime};

use frame_system::RawOrigin;
use sp_runtime::{DispatchError, Fixed128, Permill};
use sp_std::prelude::*;

use frame_benchmarking::account;
use orml_benchmarking::runtime_benchmarks;

use margin_liquidity_pools::SwapRate;
use margin_protocol::RiskThreshold;
use module_primitives::*;

const SEED: u32 = 0;
const MAX_POOL_INDEX: u32 = 1000;
const MAX_SPREAD: u32 = 1000;
const MAX_SWAP_RATE: u32 = 1000;
const MAX_AMOUNT: u32 = 1000;

const EUR_USD: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};

fn create_pool(p: u32) -> Result<AccountId, DispatchError> {
	let caller: AccountId = account("caller", p, SEED);
	BaseLiquidityPoolsForMargin::create_pool(RawOrigin::Signed(caller.clone()).into())?;

	Ok(caller)
}

runtime_benchmarks! {
	{ Runtime, margin_liquidity_pools }

	_ {
		let p in 1 .. MAX_POOL_INDEX => ();
		let s in 1 .. MAX_SPREAD => ();
		let r in 1 .. MAX_SWAP_RATE => ();
		let a in 1 .. MAX_AMOUNT => ();
	}

	set_spread {
		let p in ...;
		let s in ...;
		let caller = create_pool(p)?;
	}: _(RawOrigin::Signed(caller), 0, EUR_USD, s.into(), s.into())

	set_enabled_trades {
		let p in ...;
		let caller = create_pool(p)?;
	}: _(RawOrigin::Signed(caller), 0, EUR_USD, Leverages::all())

	set_swap_rate {
		let p in ...;
		let r in ...;
		let _ = create_pool(p)?;
		let swap_rate = SwapRate {
			long: Fixed128::from_parts(r.into()),
			short: Fixed128::from_parts(r.into()),
		};
	}: _(RawOrigin::Root, EUR_USD, swap_rate)

	set_additional_swap {
		let p in ...;
		let r in ...;
		let caller = create_pool(p)?;
		let rate = Fixed128::from_parts(r.into());
	}: _(RawOrigin::Signed(caller), 0, rate)

	set_max_spread {
		let s in ...;
	}: _(RawOrigin::Root, EUR_USD, s.into())

	set_accumulate {
		let frequency: BlockNumber = 10u32;
		let offset: BlockNumber = 1u32;
	}: _(RawOrigin::Root, EUR_USD, frequency, offset)

	enable_trading_pair {
	}: _(RawOrigin::Root, EUR_USD)

	disable_trading_pair {
	}: _(RawOrigin::Root, EUR_USD)

	liquidity_pool_enable_trading_pair {
		let p in ...;
		let caller = create_pool(p)?;
		let threshold = RiskThreshold {
			margin_call: Permill::from_percent(5),
			stop_out: Permill::from_percent(2),
		};
		MarginProtocol::set_trading_pair_risk_threshold(
			RawOrigin::Root.into(),
			EUR_USD,
			Some(threshold.clone()),
			Some(threshold.clone()),
			Some(threshold.clone()),
		)?;
		MarginLiquidityPools::enable_trading_pair(RawOrigin::Root.into(), EUR_USD)?;
	}: _(RawOrigin::Signed(caller), 0, EUR_USD)

	liquidity_pool_disable_trading_pair {
		let p in ...;
		let caller = create_pool(p)?;
		MarginLiquidityPools::enable_trading_pair(RawOrigin::Root.into(), EUR_USD)?;
		MarginLiquidityPools::liquidity_pool_enable_trading_pair(
			RawOrigin::Signed(caller.clone()).into(),
			0,
			EUR_USD,
		)?;
	}: _(RawOrigin::Signed(caller), 0, EUR_USD)

	set_default_min_leveraged_amount {
		let a in ...;
	}: _(RawOrigin::Root, dollars(a))

	set_min_leveraged_amount {
		let p in ...;
		let a in ...;
		let caller = create_pool(p)?;
		MarginLiquidityPools::set_default_min_leveraged_amount(
			RawOrigin::Root.into(),
			a.into(),
		)?;
	}: _(RawOrigin::Signed(caller), 0, a.into())
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
	fn set_enabled_trades() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_enabled_trades());
		});
	}

	#[test]
	fn set_swap_rate() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_swap_rate());
		});
	}

	#[test]
	fn set_additional_swap() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_additional_swap());
		});
	}

	#[test]
	fn set_max_spread() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_max_spread());
		});
	}

	#[test]
	fn set_accumulate() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_accumulate());
		});
	}

	#[test]
	fn enable_trading_pair() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_enable_trading_pair());
		});
	}

	#[test]
	fn disable_trading_pair() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_disable_trading_pair());
		});
	}

	#[test]
	fn liquidity_pool_enable_trading_pair() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_liquidity_pool_enable_trading_pair());
		});
	}

	#[test]
	fn liquidity_pool_disable_trading_pair() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_liquidity_pool_disable_trading_pair());
		});
	}

	#[test]
	fn set_default_min_leveraged_amount() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_default_min_leveraged_amount());
		});
	}

	#[test]
	fn set_min_leveraged_amount() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_min_leveraged_amount());
		});
	}
}
