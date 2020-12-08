//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(clippy::unnecessary_cast)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

use sp_std::marker::PhantomData;

pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> synthetic_protocol::WeightInfo for WeightInfo<T> {
	fn mint() -> Weight {
		(506_992_000 as Weight)
			.saturating_add(DbWeight::get().reads(22 as Weight))
			.saturating_add(DbWeight::get().writes(9 as Weight))
	}
	fn redeem() -> Weight {
		(661_365_000 as Weight)
			.saturating_add(DbWeight::get().reads(22 as Weight))
			.saturating_add(DbWeight::get().writes(9 as Weight))
	}
	fn liquidate() -> Weight {
		(567_526_000 as Weight)
			.saturating_add(DbWeight::get().reads(20 as Weight))
			.saturating_add(DbWeight::get().writes(8 as Weight))
	}
	fn add_collateral() -> Weight {
		(271_474_000 as Weight)
			.saturating_add(DbWeight::get().reads(9 as Weight))
			.saturating_add(DbWeight::get().writes(7 as Weight))
	}
	fn withdraw_collateral() -> Weight {
		(411_939_000 as Weight)
			.saturating_add(DbWeight::get().reads(20 as Weight))
			.saturating_add(DbWeight::get().writes(7 as Weight))
	}
}
