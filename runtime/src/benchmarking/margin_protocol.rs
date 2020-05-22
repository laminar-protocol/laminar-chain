use super::utils::{dollars, set_ausd_balance, set_price};
use crate::{AccountId, BaseLiquidityPoolsForMargin, MarginLiquidityPools, MarginProtocol, Oracle, Price, Runtime};

use frame_support::{assert_ok, traits::ChangeMembers};
use frame_system::RawOrigin;
use sp_runtime::{Fixed128, Permill};
use sp_std::prelude::*;

use frame_benchmarking::account;
use orml_benchmarking::runtime_benchmarks;

use margin_protocol::RiskThreshold;
use module_primitives::*;

const SEED: u32 = 0;
const MAX_TRADER_INDEX: u32 = 1000;
const MAX_POOL_OWNER_INDEX: u32 = 1000;
const MAX_DOLLARS: u32 = 1000;

const EUR_USD: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};

fn create_pool(p: u32) -> AccountId {
	let owner: AccountId = account("owner", p, SEED);
	assert_ok!(BaseLiquidityPoolsForMargin::create_pool(
		RawOrigin::Signed(owner.clone()).into()
	));

	let threshold = RiskThreshold {
		margin_call: Permill::from_percent(5),
		stop_out: Permill::from_percent(2),
	};
	assert_ok!(MarginProtocol::set_trading_pair_risk_threshold(
		RawOrigin::Root.into(),
		EUR_USD,
		Some(threshold.clone()),
		Some(threshold.clone()),
		Some(threshold),
	));
	assert_ok!(MarginLiquidityPools::enable_trading_pair(
		RawOrigin::Root.into(),
		EUR_USD
	));
	assert_ok!(MarginLiquidityPools::set_enabled_trades(
		RawOrigin::Signed(owner.clone()).into(),
		0,
		EUR_USD,
		Leverages::all()
	));
	assert_ok!(MarginLiquidityPools::liquidity_pool_enable_trading_pair(
		RawOrigin::Signed(owner.clone()).into(),
		0,
		EUR_USD
	));

	owner
}

fn deposit_balance(who: &AccountId, balance: Balance) {
	// extra dollar for fees
	set_ausd_balance(&who, balance + dollars(1u128));
	assert_ok!(MarginProtocol::deposit(
		RawOrigin::Signed(who.clone()).into(),
		0,
		balance
	));
}

fn add_liquidity(owner: &AccountId, liquidity: Balance) {
	set_ausd_balance(owner, liquidity + dollars(1u128));
	assert_ok!(BaseLiquidityPoolsForMargin::deposit_liquidity(
		RawOrigin::Signed(owner.clone()).into(),
		0,
		liquidity
	));
}

fn set_up_oracle() {
	<Oracle as ChangeMembers<_>>::change_members_sorted(
		&vec![],
		&vec![],
		&vec![AccountId::from([100u8; 32]), AccountId::from([101u8; 32])],
	);
}

runtime_benchmarks! {
	{ Runtime, margin_protocol }

	_ {
		let t in 1 .. MAX_TRADER_INDEX => ();
		let p in 1 .. MAX_POOL_OWNER_INDEX => ();
		let d in 1 .. MAX_DOLLARS => ();
	}

	deposit {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p);

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		set_ausd_balance(&trader, balance + dollars(1u128));
	}: _(RawOrigin::Signed(trader.clone()), 0, balance)
	verify {
		assert_eq!(MarginProtocol::balances(&trader, 0), Fixed128::from_natural(d.into()));
	}

	withdraw {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p);

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance);
	}: _(RawOrigin::Signed(trader.clone()), 0, balance)
	verify {
		assert_eq!(MarginProtocol::balances(&trader, 0), Fixed128::zero());
	}

	open_position {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p);

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance);

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity);

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))]);
	}: _(RawOrigin::Signed(trader), 0, EUR_USD, Leverage::LongTwo, balance, Price::from_natural(2))

	// `open_position` when there is already ten positions in pool
	open_position_with_ten_in_pool {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p);

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance);

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity);

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))]);

		for _ in 0..10 {
			assert_ok!(MarginProtocol::open_position(
				RawOrigin::Signed(trader.clone()).into(),
				0,
				EUR_USD,
				Leverage::LongTwo,
				balance / 10,
				Price::from_natural(2)
			));
		}
	}: open_position(RawOrigin::Signed(trader), 0, EUR_USD, Leverage::LongTwo, balance, Price::from_natural(2))

	close_position {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p);

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance);

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity);

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))]);

		assert_ok!(MarginProtocol::open_position(
			RawOrigin::Signed(trader.clone()).into(),
			0,
			EUR_USD,
			Leverage::LongTwo,
			balance / 10,
			Price::from_natural(2)
		));
	}: _(RawOrigin::Signed(trader), 0, Price::zero())

	// `close_position` when there is already ten positions in pool
	close_position_with_ten_in_pool {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p);

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance);

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity);

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))]);

		for _ in 0..10 {
			assert_ok!(MarginProtocol::open_position(
				RawOrigin::Signed(trader.clone()).into(),
				0,
				EUR_USD,
				Leverage::LongTwo,
				balance / 10,
				Price::from_natural(2)
			));
		}
	}: close_position(RawOrigin::Signed(trader), 0, Price::zero())
}

#[cfg(test)]
mod tests {
	use super::*;

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
	fn deposit() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_deposit());
		});
	}

	#[test]
	fn withdraw() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_withdraw());
		});
	}

	#[test]
	fn open_position() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_open_position());
		});
	}

	#[test]
	fn open_position_with_ten_in_pool() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_open_position_with_ten_in_pool());
		});
	}

	#[test]
	fn close_position() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_close_position());
		});
	}

	#[test]
	fn close_position_with_ten_in_pool() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_close_position_with_ten_in_pool());
		});
	}
}
