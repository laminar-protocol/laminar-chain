//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn deposit() -> Weight {
		(159_232_000 as Weight)
			.saturating_add(DbWeight::get().reads(7 as Weight))
			.saturating_add(DbWeight::get().writes(5 as Weight))
	}
	fn withdraw() -> Weight {
		(384_400_000 as Weight)
			.saturating_add(DbWeight::get().reads(8 as Weight))
			.saturating_add(DbWeight::get().writes(5 as Weight))
	}
	fn open_position() -> Weight {
		(1_172_175_000 as Weight)
			.saturating_add(DbWeight::get().reads(26 as Weight))
			.saturating_add(DbWeight::get().writes(7 as Weight))
	}
	fn open_position_with_ten_in_pool() -> Weight {
		(4_786_901_000 as Weight)
			.saturating_add(DbWeight::get().reads(46 as Weight))
			.saturating_add(DbWeight::get().writes(7 as Weight))
	}
	fn close_position() -> Weight {
		(535_587_000 as Weight)
			.saturating_add(DbWeight::get().reads(20 as Weight))
			.saturating_add(DbWeight::get().writes(7 as Weight))
	}
	fn close_position_with_ten_in_pool() -> Weight {
		(2_004_307_000 as Weight)
			.saturating_add(DbWeight::get().reads(38 as Weight))
			.saturating_add(DbWeight::get().writes(7 as Weight))
	}
	fn trader_margin_call() -> Weight {
		(439_221_000 as Weight)
			.saturating_add(DbWeight::get().reads(21 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn trader_become_safe() -> Weight {
		(481_074_000 as Weight)
			.saturating_add(DbWeight::get().reads(21 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn trader_stop_out() -> Weight {
		(1_476_823_000 as Weight)
			.saturating_add(DbWeight::get().reads(25 as Weight))
			.saturating_add(DbWeight::get().writes(10 as Weight))
	}
	fn liquidity_pool_margin_call() -> Weight {
		(532_767_000 as Weight)
			.saturating_add(DbWeight::get().reads(19 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn liquidity_pool_become_safe() -> Weight {
		(525_795_000 as Weight)
			.saturating_add(DbWeight::get().reads(19 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn liquidity_pool_force_close() -> Weight {
		(1_485_114_000 as Weight)
			.saturating_add(DbWeight::get().reads(28 as Weight))
			.saturating_add(DbWeight::get().writes(10 as Weight))
	}
	fn set_trading_pair_risk_threshold() -> Weight {
		(73_093_000 as Weight)
			.saturating_add(DbWeight::get().reads(5 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
}
