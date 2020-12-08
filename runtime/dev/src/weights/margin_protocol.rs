//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

use sp_std::marker::PhantomData;

pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> margin_protocol::WeightInfo for WeightInfo<T> {
	fn deposit() -> Weight {
		(159_232_000_u64)
			.saturating_add(DbWeight::get().reads(7_u64))
			.saturating_add(DbWeight::get().writes(5_u64))
	}
	fn withdraw() -> Weight {
		(384_400_000_u64)
			.saturating_add(DbWeight::get().reads(8_u64))
			.saturating_add(DbWeight::get().writes(5_u64))
	}
	fn open_position() -> Weight {
		(1_172_175_000_u64)
			.saturating_add(DbWeight::get().reads(26_u64))
			.saturating_add(DbWeight::get().writes(7_u64))
	}
	fn open_position_with_ten_in_pool() -> Weight {
		(4_786_901_000_u64)
			.saturating_add(DbWeight::get().reads(46_u64))
			.saturating_add(DbWeight::get().writes(7_u64))
	}
	fn close_position() -> Weight {
		(535_587_000_u64)
			.saturating_add(DbWeight::get().reads(20_u64))
			.saturating_add(DbWeight::get().writes(7_u64))
	}
	fn close_position_with_ten_in_pool() -> Weight {
		(2_004_307_000_u64)
			.saturating_add(DbWeight::get().reads(38_u64))
			.saturating_add(DbWeight::get().writes(7_u64))
	}
	fn trader_margin_call() -> Weight {
		(439_221_000_u64)
			.saturating_add(DbWeight::get().reads(21_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn trader_become_safe() -> Weight {
		(481_074_000_u64)
			.saturating_add(DbWeight::get().reads(21_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn trader_stop_out() -> Weight {
		(1_476_823_000_u64)
			.saturating_add(DbWeight::get().reads(25_u64))
			.saturating_add(DbWeight::get().writes(10_u64))
	}
	fn liquidity_pool_margin_call() -> Weight {
		(532_767_000_u64)
			.saturating_add(DbWeight::get().reads(19_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn liquidity_pool_become_safe() -> Weight {
		(525_795_000_u64)
			.saturating_add(DbWeight::get().reads(19_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn liquidity_pool_force_close() -> Weight {
		(1_485_114_000_u64)
			.saturating_add(DbWeight::get().reads(28_u64))
			.saturating_add(DbWeight::get().writes(10_u64))
	}
	fn set_trading_pair_risk_threshold() -> Weight {
		(73_093_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
}
