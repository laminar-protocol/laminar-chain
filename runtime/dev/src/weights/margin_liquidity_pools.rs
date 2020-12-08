//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

use sp_std::marker::PhantomData;

pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> margin_liquidity_pools::WeightInfo for WeightInfo<T> {
	fn set_spread() -> Weight {
		(86_440_000_u64)
			.saturating_add(DbWeight::get().reads(6_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_enabled_leverages() -> Weight {
		(71_972_000_u64)
			.saturating_add(DbWeight::get().reads(6_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_swap_rate() -> Weight {
		(63_306_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_additional_swap_rate() -> Weight {
		(70_844_000_u64)
			.saturating_add(DbWeight::get().reads(6_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_max_spread() -> Weight {
		(63_945_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_accumulate_config() -> Weight {
		(64_648_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn enable_trading_pair() -> Weight {
		(62_586_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn disable_trading_pair() -> Weight {
		(65_692_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn liquidity_pool_enable_trading_pair() -> Weight {
		(167_421_000_u64)
			.saturating_add(DbWeight::get().reads(9_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn liquidity_pool_disable_trading_pair() -> Weight {
		(125_038_000_u64)
			.saturating_add(DbWeight::get().reads(6_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_default_min_leveraged_amount() -> Weight {
		(68_719_000_u64)
			.saturating_add(DbWeight::get().reads(4_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_min_leveraged_amount() -> Weight {
		(119_069_000_u64)
			.saturating_add(DbWeight::get().reads(6_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn on_initialize(r: u32, w: u32) -> Weight {
		(245_763_000_u64)
			.saturating_add((35_620_000_u64).saturating_mul(r as Weight))
			.saturating_add((92_617_000_u64).saturating_mul(w as Weight))
			.saturating_add(DbWeight::get().reads(11_u64))
			.saturating_add(DbWeight::get().reads((1_u64).saturating_mul(r as Weight)))
			.saturating_add(DbWeight::get().reads((1_u64).saturating_mul(w as Weight)))
			.saturating_add(DbWeight::get().writes((1_u64).saturating_mul(w as Weight)))
	}
}
