//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

use sp_std::marker::PhantomData;

pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> synthetic_tokens::WeightInfo for WeightInfo<T> {
	fn set_extreme_ratio() -> Weight {
		(56_349_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_liquidation_ratio() -> Weight {
		(57_010_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn set_collateral_ratio() -> Weight {
		(66_234_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
}
