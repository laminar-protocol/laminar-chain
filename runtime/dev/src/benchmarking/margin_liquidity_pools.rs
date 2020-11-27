use super::utils::dollars;
use crate::{
	AccountId, BaseLiquidityPoolsForMargin, MarginLiquidityPools, MarginProtocol, Origin, Runtime, StorageValue,
	SyntheticCurrencyIds, System,
};

use frame_support::traits::OnInitialize;
use frame_system::RawOrigin;
use margin_liquidity_pools::ONE_MINUTE;
use sp_runtime::{DispatchError, FixedI128, Permill};
use sp_std::prelude::*;

use frame_benchmarking::account;
use orml_benchmarking::runtime_benchmarks;

use margin_protocol::RiskThreshold;
use primitives::*;

const SEED: u32 = 0;

const EUR_USD: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};

fn create_pool() -> Result<AccountId, DispatchError> {
	let caller: AccountId = account("caller", 0, SEED);
	BaseLiquidityPoolsForMargin::create_pool(RawOrigin::Signed(caller.clone()).into())?;

	Ok(caller)
}

runtime_benchmarks! {
	{ Runtime, margin_liquidity_pools }

	_ {}

	set_spread {
		let caller = create_pool()?;
	}: _(RawOrigin::Signed(caller), 0, EUR_USD, Price::from_inner(1u128), Price::from_inner(1u128))

	set_enabled_leverages {
		let caller = create_pool()?;
	}: _(RawOrigin::Signed(caller), 0, EUR_USD, Leverages::all())

	set_swap_rate {
		let _ = create_pool()?;
		let swap_rate = SwapRate {
			long: FixedI128::from_inner(1.into()),
			short: FixedI128::from_inner(1.into()),
		};
	}: _(RawOrigin::Root, EUR_USD, swap_rate)

	set_additional_swap_rate {
		let caller = create_pool()?;
		let rate = FixedI128::from_inner(1.into());
	}: _(RawOrigin::Signed(caller), 0, rate)

	set_max_spread {
	}: _(RawOrigin::Root, EUR_USD, Price::from_inner(1u128))

	set_accumulate_config {
		let frequency = 60u64;
		let offset = 1u64;
	}: _(RawOrigin::Root, EUR_USD, frequency, offset)

	enable_trading_pair {
	}: _(RawOrigin::Root, EUR_USD)

	disable_trading_pair {
	}: _(RawOrigin::Root, EUR_USD)

	liquidity_pool_enable_trading_pair {
		let caller = create_pool()?;
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
		let caller = create_pool()?;
		MarginLiquidityPools::enable_trading_pair(RawOrigin::Root.into(), EUR_USD)?;
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
		MarginLiquidityPools::liquidity_pool_enable_trading_pair(
			RawOrigin::Signed(caller.clone()).into(),
			0,
			EUR_USD,
		)?;
	}: _(RawOrigin::Signed(caller), 0, EUR_USD)

	set_default_min_leveraged_amount {
	}: _(RawOrigin::Root, dollars(100u128))

	set_min_leveraged_amount {
		let caller = create_pool()?;
		MarginLiquidityPools::set_default_min_leveraged_amount(
			RawOrigin::Root.into(),
			1u128.into(),
		)?;
	}: _(RawOrigin::Signed(caller), 0, 10u128.into())

	on_initialize {
		let r in 0 .. SyntheticCurrencyIds::get().len().saturating_sub(1) as u32;
		let w in 0 .. 2;
		let currency_ids = SyntheticCurrencyIds::get();

		for i in 0 .. r {
			let currency_id = currency_ids[i as usize];
			let pair: TradingPair = TradingPair {
				base: currency_id,
				quote: CurrencyId::AUSD,
			};

			if i < w {
				MarginLiquidityPools::set_accumulate_config(Origin::root(), pair, ONE_MINUTE, 0u64)?;
			} else {
				// accumulate is not executed
				MarginLiquidityPools::set_accumulate_config(Origin::root(), pair, ONE_MINUTE * 10, 0u64)?;
			}
		}
		System::set_block_number(1);
		pallet_timestamp::Now::<Runtime>::put(ONE_MINUTE * 1000); // 60_000ms
	}: {
		MarginLiquidityPools::on_initialize(System::block_number());
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
	fn set_spread() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_spread());
		});
	}

	#[test]
	fn set_enabled_leverages() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_enabled_leverages());
		});
	}

	#[test]
	fn set_swap_rate() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_swap_rate());
		});
	}

	#[test]
	fn set_additional_swap_rate() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_additional_swap_rate());
		});
	}

	#[test]
	fn set_max_spread() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_max_spread());
		});
	}

	#[test]
	fn set_accumulate_config() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_accumulate_config());
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

	#[test]
	fn on_initialize() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_on_initialize());
		});
	}
}
