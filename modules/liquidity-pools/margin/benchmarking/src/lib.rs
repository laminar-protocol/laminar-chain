//! Margin liquidity pools benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use sp_arithmetic::Fixed128;

use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;

use module_base_liquidity_pools::{Instance1, Module as BaseLiquidityPools, Trait as BaseLiquidityPoolsTrait};
use module_margin_liquidity_pools::*;
use module_margin_liquidity_pools::{Module as MarginLiquidityPools, Trait as MarginLiquidityPoolsTrait};
use primitives::{CurrencyId, Leverages, TradingPair};

pub struct Module<T: Trait>(MarginLiquidityPools<T>);

pub trait BaseLiquidityPoolsForMarginTrait: BaseLiquidityPoolsTrait<Instance1> {}
pub trait Trait: MarginLiquidityPoolsTrait + BaseLiquidityPoolsForMarginTrait {}

type BaseLiquidityPoolsForMargin<T> = BaseLiquidityPools<T, Instance1>;

const SEED: u32 = 0;
const MAX_POOL_INDEX: u32 = 1000;
const EUR_USD: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};

fn create_pool<T: Trait>(p: u32) -> T::AccountId {
	let caller: T::AccountId = account("caller", p, SEED);
	let _ = <BaseLiquidityPoolsForMargin<T>>::create_pool(RawOrigin::Signed(caller.clone()).into());

	caller
}

benchmarks! {
	_ {
		let p in 1 .. MAX_POOL_INDEX => ();
	}

	set_spread {
		let p in ...;
		let caller = create_pool::<T>(p);
	}: _(RawOrigin::Signed(caller), 0, EUR_USD, 100, 100)

	set_enabled_trades {
		let p in ...;
		let caller = create_pool::<T>(p);
	}: _(RawOrigin::Signed(caller), 0, EUR_USD, Leverages::all())

	set_swap_rate {
		let p in ...;
		let _ = create_pool::<T>(p);
		let swap_rate = SwapRate {
			long: Fixed128::from_natural(2),
			short: Fixed128::from_natural(2),
		};
	}: _(RawOrigin::Root, EUR_USD, swap_rate)

	set_additional_swap {
		let p in ...;
		let caller = create_pool::<T>(p);
		let rate = Fixed128::from_natural(1);
	}: _(RawOrigin::Signed(caller), 0, rate)

	set_max_spread {
		let p in ...;
	}: _(RawOrigin::Root, EUR_USD, 200)

	set_accumulate {
		let p in ...;
		let frequency: T::BlockNumber = 10.into();
		let offset: T::BlockNumber = 1.into();
	}: _(RawOrigin::Root, EUR_USD, frequency, offset)

	enable_trading_pair {
		let p in ...;
	}: _(RawOrigin::Root, EUR_USD)

	disable_trading_pair {
		let p in ...;
	}: _(RawOrigin::Root, EUR_USD)

	liquidity_pool_enable_trading_pair {
		let p in ...;
		let caller = create_pool::<T>(p);
		let _ = <MarginLiquidityPools<T>>::enable_trading_pair(RawOrigin::Root.into(), EUR_USD);
	}: _(RawOrigin::Signed(caller), 0, EUR_USD)

	liquidity_pool_disable_trading_pair {
		let p in ...;
		let caller = create_pool::<T>(p);
		let _ = <MarginLiquidityPools<T>>::enable_trading_pair(RawOrigin::Root.into(), EUR_USD);
		let _ = <MarginLiquidityPools<T>>::liquidity_pool_enable_trading_pair(
			RawOrigin::Signed(caller.clone()).into(),
			0,
			EUR_USD,
		);
	}: _(RawOrigin::Signed(caller), 0, EUR_USD)

	set_default_min_leveraged_amount {
		let p in ...;
	}: _(RawOrigin::Root, 100)

	set_min_leveraged_amount {
		let p in ...;
		let caller = create_pool::<T>(p);
		let _ = <MarginLiquidityPools<T>>::set_default_min_leveraged_amount(
			RawOrigin::Root.into(),
			100,
		);
	}: _(RawOrigin::Signed(caller), 0, 200)
}
