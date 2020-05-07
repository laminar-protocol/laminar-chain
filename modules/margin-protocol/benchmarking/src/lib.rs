//! Margin protocol benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use sp_arithmetic::traits::SaturatedConversion;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::traits::StaticLookup;

use frame_benchmarking::{account, benchmarks};
use frame_support::traits::Get;
use frame_system::{RawOrigin, Trait as SystemTrait};

use orml_oracle::{Module as OracleModule, Trait as OracleTrait};
use orml_traits::MultiCurrencyExtended;

use base_liquidity_pools::{Instance1, Module as BaseLiquidityPools, Trait as BaseLiquidityPoolsTrait};
use margin_liquidity_pools::{Module as MarginLiquidityPools, Trait as MarginLiquidityPoolsTrait};
use margin_protocol::*;
use margin_protocol::{Module as MarginProtocol, Trait as MarginProtocolTrait};
use primitives::{Balance, CurrencyId, Leverages, Price, TradingPair};

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

macro_rules! alice {
	() => {
		account("Alice", 0, 0)
	};
}

fn create_pool<T: Trait>(p: u32) -> T::AccountId {
	let owner: T::AccountId = account("owner", p, SEED);
	let _ = <BaseLiquidityPoolsForMargin<T>>::create_pool(RawOrigin::Signed(owner.clone()).into());

	owner
}

pub fn dollar(amount: u128) -> u128 {
	amount.saturating_mul(Price::accuracy())
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
		let trader: T::AccountId = alice!();
	}: _(RawOrigin::Signed(trader), 0, dollar(100))

	withdraw {
		let t in ...;
		let p in ...;
		let pool_owner = create_pool::<T>(p);
		let trader: T::AccountId = alice!();
		<MarginProtocol<T>>::deposit(RawOrigin::Signed(trader.clone()).into(), 0, dollar(100));
	}: _(RawOrigin::Signed(trader), 0, dollar(100))
}
