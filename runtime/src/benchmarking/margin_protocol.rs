use super::utils::{dollars, lookup_of_account, set_ausd_balance, set_price};
use crate::{AccountId, BaseLiquidityPoolsForMargin, MarginLiquidityPools, MarginProtocol, Oracle, Price, Runtime};

use frame_support::traits::ChangeMembers;
use frame_system::RawOrigin;
use sp_runtime::{DispatchError, DispatchResult, Fixed128, Permill};
use sp_std::prelude::*;

use frame_benchmarking::account;
use orml_benchmarking::runtime_benchmarks;

use margin_protocol::RiskThreshold;
use module_primitives::*;

const SEED: u32 = 0;
const MAX_TRADER_INDEX: u32 = 1000;
const MAX_POOL_OWNER_INDEX: u32 = 1000;
const MAX_DOLLARS: u32 = 1000;
const MAX_THRESHOLD: u32 = 100;

const EUR_USD: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};

fn create_pool(p: u32) -> Result<AccountId, DispatchError> {
	let owner: AccountId = account("owner", p, SEED);
	BaseLiquidityPoolsForMargin::create_pool(RawOrigin::Signed(owner.clone()).into())?;

	let threshold = RiskThreshold {
		margin_call: Permill::from_percent(5),
		stop_out: Permill::from_percent(2),
	};
	MarginProtocol::set_trading_pair_risk_threshold(
		RawOrigin::Root.into(),
		EUR_USD,
		Some(threshold.clone()),
		Some(threshold.clone()),
		Some(threshold),
	)?;
	MarginLiquidityPools::set_spread(RawOrigin::Signed(owner.clone()).into(), 0, EUR_USD, 0, 0)?;
	MarginLiquidityPools::enable_trading_pair(RawOrigin::Root.into(), EUR_USD)?;
	MarginLiquidityPools::set_enabled_trades(RawOrigin::Signed(owner.clone()).into(), 0, EUR_USD, Leverages::all())?;
	MarginLiquidityPools::liquidity_pool_enable_trading_pair(RawOrigin::Signed(owner.clone()).into(), 0, EUR_USD)?;

	Ok(owner)
}

fn deposit_balance(who: &AccountId, balance: Balance) -> DispatchResult {
	// extra dollar for fees
	set_ausd_balance(&who, balance + dollars(1u128))?;
	MarginProtocol::deposit(RawOrigin::Signed(who.clone()).into(), 0, balance)
}

fn add_liquidity(owner: &AccountId, liquidity: Balance) -> DispatchResult {
	set_ausd_balance(owner, liquidity + dollars(1u128))?;
	BaseLiquidityPoolsForMargin::deposit_liquidity(RawOrigin::Signed(owner.clone()).into(), 0, liquidity)
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
		let d in 100 .. MAX_DOLLARS => ();
		let h in 1 .. MAX_THRESHOLD => ();
	}

	deposit {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		set_ausd_balance(&trader, balance + dollars(1u128))?;
	}: _(RawOrigin::Signed(trader.clone()), 0, balance)
	verify {
		assert_eq!(MarginProtocol::balances(&trader, 0), Fixed128::from_natural(d.into()));
	}

	withdraw {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;
	}: _(RawOrigin::Signed(trader.clone()), 0, balance)
	verify {
		assert_eq!(MarginProtocol::balances(&trader, 0), Fixed128::zero());
	}

	open_position {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;
	}: _(RawOrigin::Signed(trader), 0, EUR_USD, Leverage::LongTwo, balance, Price::from_natural(2))

	// `open_position` when there is already ten positions in pool
	open_position_with_ten_in_pool {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;

		for _ in 0..10 {
			MarginProtocol::open_position(
				RawOrigin::Signed(trader.clone()).into(),
				0,
				EUR_USD,
				Leverage::LongTwo,
				balance / 10,
				Price::from_natural(2)
			)?;
		}
	}: open_position(RawOrigin::Signed(trader), 0, EUR_USD, Leverage::LongTwo, balance, Price::from_natural(2))

	close_position {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;

		MarginProtocol::open_position(
			RawOrigin::Signed(trader.clone()).into(),
			0,
			EUR_USD,
			Leverage::LongTwo,
			balance,
			Price::from_natural(2)
		)?;
	}: _(RawOrigin::Signed(trader), 0, Price::zero())

	// `close_position` when there is already ten positions in pool
	close_position_with_ten_in_pool {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;

		for _ in 0..10 {
			MarginProtocol::open_position(
				RawOrigin::Signed(trader.clone()).into(),
				0,
				EUR_USD,
				Leverage::LongTwo,
				balance / 10,
				Price::from_natural(2)
			)?;
		}
	}: close_position(RawOrigin::Signed(trader), 0, Price::zero())

	trader_margin_call {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(2))])?;

		MarginProtocol::open_position(
			RawOrigin::Signed(trader.clone()).into(),
			0,
			EUR_USD,
			Leverage::LongTwo,
			balance,
			Price::from_natural(3)
		)?;

		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;
	}: _(RawOrigin::None, lookup_of_account(trader.clone()), 0)
	verify {
		assert_eq!(MarginProtocol::margin_called_traders(&trader, 0), Some(true));
	}

	trader_become_safe {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(2))])?;

		MarginProtocol::open_position(
			RawOrigin::Signed(trader.clone()).into(),
			0,
			EUR_USD,
			Leverage::LongTwo,
			balance,
			Price::from_natural(3)
		)?;

		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;
		MarginProtocol::trader_margin_call(
			RawOrigin::None.into(),
			lookup_of_account(trader.clone()),
			0
		)?;

		assert_eq!(MarginProtocol::margin_called_traders(&trader, 0), Some(true));

		set_price(vec![(CurrencyId::FEUR, Price::from_natural(2))])?;
	}: _(RawOrigin::None, lookup_of_account(trader.clone()), 0)
	verify {
		assert_eq!(MarginProtocol::margin_called_traders(&trader, 0), None);
	}

	trader_stop_out {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(2))])?;

		MarginProtocol::open_position(
			RawOrigin::Signed(trader.clone()).into(),
			0,
			EUR_USD,
			Leverage::LongTwo,
			balance,
			Price::from_natural(3)
		)?;
		assert_eq!(MarginProtocol::positions_by_trader(&trader, (0, 0)), Some(true));

		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;
	}: _(RawOrigin::None, lookup_of_account(trader.clone()), 0)
	verify {
		assert_eq!(MarginProtocol::positions_by_trader(&trader, (0, 0)), None);
	}

	liquidity_pool_margin_call {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;

		MarginProtocol::open_position(
			RawOrigin::Signed(trader.clone()).into(),
			0,
			EUR_USD,
			Leverage::LongTwo,
			balance,
			Price::from_natural(2)
		)?;

		set_price(vec![(CurrencyId::FEUR, Price::from_natural(2))])?;
	}: _(RawOrigin::None, 0)
	verify {
		assert_eq!(MarginProtocol::margin_called_pools(0), Some(true))
	}

	liquidity_pool_become_safe {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;

		MarginProtocol::open_position(
			RawOrigin::Signed(trader.clone()).into(),
			0,
			EUR_USD,
			Leverage::LongTwo,
			balance,
			Price::from_natural(2)
		)?;

		set_price(vec![(CurrencyId::FEUR, Price::from_natural(2))])?;
		MarginProtocol::liquidity_pool_margin_call(RawOrigin::None.into(), 0)?;
		assert_eq!(MarginProtocol::margin_called_pools(0), Some(true));

		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;
	}: _(RawOrigin::None, 0)
	verify {
		assert_eq!(MarginProtocol::margin_called_pools(0), None)
	}

	liquidity_pool_force_close {
		let t in ...;
		let p in ...;
		let d in ...;

		let pool_owner = create_pool(p)?;

		let trader: AccountId = account("trader", t, SEED);
		let balance = dollars(d);
		deposit_balance(&trader, balance)?;

		let liquidity = balance;
		add_liquidity(&pool_owner, liquidity)?;

		set_up_oracle();
		set_price(vec![(CurrencyId::FEUR, Price::from_natural(1))])?;

		MarginProtocol::open_position(
			RawOrigin::Signed(trader.clone()).into(),
			0,
			EUR_USD,
			Leverage::LongTwo,
			balance,
			Price::from_natural(2)
		)?;
		assert_eq!(MarginProtocol::positions_by_pool(0, (EUR_USD, 0)), Some(true));

		set_price(vec![(CurrencyId::FEUR, Price::from_natural(2))])?;
	}: _(RawOrigin::None, 0)
	verify {
		assert_eq!(MarginProtocol::positions_by_pool(0, (EUR_USD, 0)), None);
	}

	set_trading_pair_risk_threshold {
		let p in ...;
		let h in ...;


		let pool_owner: AccountId = account("owner", p, SEED);
		BaseLiquidityPoolsForMargin::create_pool(
			RawOrigin::Signed(pool_owner.clone()).into()
		)?;

		let threshold = RiskThreshold {
			margin_call: Permill::from_percent(h),
			stop_out: Permill::from_percent(h),
		};
	}: _(RawOrigin::Root, EUR_USD, Some(threshold.clone()), Some(threshold.clone()), Some(threshold.clone()))
	verify {
		assert_eq!(MarginProtocol::trader_risk_threshold(EUR_USD), Some(threshold.clone()));
		assert_eq!(MarginProtocol::liquidity_pool_enp_threshold(EUR_USD), Some(threshold.clone()));
		assert_eq!(MarginProtocol::liquidity_pool_ell_threshold(EUR_USD), Some(threshold));
	}
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

	#[test]
	fn trader_margin_call() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_trader_margin_call());
		});
	}

	#[test]
	fn trader_become_safe() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_trader_become_safe());
		});
	}

	#[test]
	fn trader_stop_out() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_trader_stop_out());
		});
	}

	#[test]
	fn liquidity_pool_margin_call() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_liquidity_pool_margin_call());
		});
	}

	#[test]
	fn liquidity_pool_become_safe() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_liquidity_pool_become_safe());
		});
	}

	#[test]
	fn liquidity_pool_force_close() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_liquidity_pool_force_close());
		});
	}

	#[test]
	fn set_trading_pair_risk_threshold() {
		new_test_ext().execute_with(|| {
			assert_ok!(test_benchmark_set_trading_pair_risk_threshold());
		});
	}
}
