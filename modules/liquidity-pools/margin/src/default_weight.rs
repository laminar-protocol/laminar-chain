//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

impl crate::WeightInfo for () {
	fn set_spread() -> Weight {
		(86_440_000 as Weight)
			.saturating_add(DbWeight::get().reads(6 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn set_enabled_leverages() -> Weight {
		(71_972_000 as Weight)
			.saturating_add(DbWeight::get().reads(6 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn set_swap_rate() -> Weight {
		(63_306_000 as Weight)
			.saturating_add(DbWeight::get().reads(5 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn set_additional_swap_rate() -> Weight {
		(70_844_000 as Weight)
			.saturating_add(DbWeight::get().reads(6 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn set_max_spread() -> Weight {
		(63_945_000 as Weight)
			.saturating_add(DbWeight::get().reads(5 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn set_accumulate_config() -> Weight {
		(64_648_000 as Weight)
			.saturating_add(DbWeight::get().reads(5 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn enable_trading_pair() -> Weight {
		(62_586_000 as Weight)
			.saturating_add(DbWeight::get().reads(5 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn disable_trading_pair() -> Weight {
		(65_692_000 as Weight)
			.saturating_add(DbWeight::get().reads(5 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn liquidity_pool_enable_trading_pair() -> Weight {
		(167_421_000 as Weight)
			.saturating_add(DbWeight::get().reads(9 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn liquidity_pool_disable_trading_pair() -> Weight {
		(125_038_000 as Weight)
			.saturating_add(DbWeight::get().reads(6 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn set_default_min_leveraged_amount() -> Weight {
		(68_719_000 as Weight)
			.saturating_add(DbWeight::get().reads(4 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn set_min_leveraged_amount() -> Weight {
		(119_069_000 as Weight)
			.saturating_add(DbWeight::get().reads(6 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn on_initialize(r: u32, w: u32) -> Weight {
		(245_763_000 as Weight)
			.saturating_add((35_620_000 as Weight).saturating_mul(r as Weight))
			.saturating_add((92_617_000 as Weight).saturating_mul(w as Weight))
			.saturating_add(DbWeight::get().reads(11 as Weight))
			.saturating_add(DbWeight::get().reads((1 as Weight).saturating_mul(r as Weight)))
			.saturating_add(DbWeight::get().reads((1 as Weight).saturating_mul(w as Weight)))
			.saturating_add(DbWeight::get().writes((1 as Weight).saturating_mul(w as Weight)))
	}
}
