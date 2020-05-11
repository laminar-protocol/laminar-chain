//! Margin protocol benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use sp_arithmetic::traits::SaturatedConversion;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::traits::StaticLookup;

use frame_benchmarking::{account, benchmarks};
use frame_system::{RawOrigin, Trait as SystemTrait};

use orml_oracle::{Module as OracleModule, Trait as OracleTrait};
use orml_traits::MultiCurrencyExtended;

use base_liquidity_pools::{Instance1, Module as BaseLiquidityPools, Trait as BaseLiquidityPoolsTrait};
use margin_liquidity_pools::{Module as MarginLiquidityPools, Trait as MarginLiquidityPoolsTrait};
use margin_protocol::*;
use margin_protocol::{Module as MarginProtocol, Trait as MarginProtocolTrait};
use primitives::{Balance, CurrencyId, Leverage, Leverages, Price, TradingPair};

pub struct Module<T: Trait>(MarginProtocol<T>);

pub trait Trait:
	OracleTrait + BaseLiquidityPoolsTrait<Instance1> + MarginLiquidityPoolsTrait + MarginProtocolTrait
{
}

type BaseLiquidityPoolsForMargin<T> = BaseLiquidityPools<T, Instance1>;

const SEED: u32 = 0;
const MAX_USER_INDEX: u32 = 1000;

const EUR_USD: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};

fn create_pool<T: Trait>(p: u32) -> T::AccountId {
	let owner: T::AccountId = account("owner", p, SEED);
	let _ = <BaseLiquidityPoolsForMargin<T>>::create_pool(RawOrigin::Signed(owner.clone()).into());

	owner
}

pub fn dollar(amount: u128) -> u128 {
	amount.saturating_mul(Price::accuracy())
}

pub fn set_balance<T: Trait>(who: &T::AccountId, amount: Balance) {
	let bench_fund: T::AccountId = account("BenchFund", 0, 0);
	let _ = <MarginProtocol<T>>::transfer_usd(&bench_fund, who, amount);
}

benchmarks! {
	_ {
		let t in 1 .. MAX_USER_INDEX => ();
		let p in 1 .. MAX_USER_INDEX => ();
	}

	deposit {
		let t in ...;
		let p in ...;

		let pool_owner = create_pool::<T>(p);

		let trader: T::AccountId = account("trader", t, SEED);
		set_balance::<T>(&trader, dollar(100));
	}: _(RawOrigin::Signed(trader), 0, dollar(100))

	withdraw {
		let t in ...;
		let p in ...;

		let pool_owner = create_pool::<T>(p);

		let trader: T::AccountId = account("trader", t, SEED);
		set_balance::<T>(&trader, dollar(100));

		let _ = <MarginProtocol<T>>::deposit(RawOrigin::Signed(trader.clone()).into(), 0, dollar(100));
	}: _(RawOrigin::Signed(trader), 0, dollar(100))

	open_position {
		let t in ...;
		let p in ...;

		let pool_owner = create_pool::<T>(p);
		let _ = <MarginLiquidityPools<T>>::set_spread(
			RawOrigin::Signed(pool_owner.clone()).into(),
			0,
			EUR_USD,
			dollar(1),
			dollar(1),
		);
		let _ = <MarginLiquidityPools<T>>::set_enabled_trades(
			RawOrigin::Signed(pool_owner.clone()).into(),
			0,
			EUR_USD,
			Leverages::all(),
		);
		set_balance::<T>(&pool_owner, dollar(100_000));

		let trader: T::AccountId = account("trader", t, SEED);
		set_balance::<T>(&trader, dollar(1000));

		let _ = <MarginProtocol<T>>::deposit(RawOrigin::Signed(trader.clone()).into(), 0, dollar(100));
	}: _(RawOrigin::Signed(trader), 0, EUR_USD, Leverage::LongTwo, dollar(1000), Price::zero())
}
