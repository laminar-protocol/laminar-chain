//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

use sp_std::marker::PhantomData;

pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> synthetic_protocol::WeightInfo for WeightInfo<T> {
	fn mint() -> Weight {
		(506_992_000_u64)
			.saturating_add(DbWeight::get().reads(22_u64))
			.saturating_add(DbWeight::get().writes(9_u64))
	}
	fn redeem() -> Weight {
		(661_365_000_u64)
			.saturating_add(DbWeight::get().reads(22_u64))
			.saturating_add(DbWeight::get().writes(9_u64))
	}
	fn liquidate() -> Weight {
		(567_526_000_u64)
			.saturating_add(DbWeight::get().reads(20_u64))
			.saturating_add(DbWeight::get().writes(8_u64))
	}
	fn add_collateral() -> Weight {
		(271_474_000_u64)
			.saturating_add(DbWeight::get().reads(9_u64))
			.saturating_add(DbWeight::get().writes(7_u64))
	}
	fn withdraw_collateral() -> Weight {
		(411_939_000_u64)
			.saturating_add(DbWeight::get().reads(20_u64))
			.saturating_add(DbWeight::get().writes(7_u64))
	}
}
