//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

use sp_std::marker::PhantomData;

pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> base_liquidity_pools::WeightInfo for WeightInfo<T> {
	fn create_pool() -> Weight {
		(62_760_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(4_u64))
	}
	fn disable_pool() -> Weight {
		(89_374_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(2_u64))
	}
	fn remove_pool() -> Weight {
		(143_453_000_u64)
			.saturating_add(DbWeight::get().reads(7_u64))
			.saturating_add(DbWeight::get().writes(4_u64))
	}
	fn deposit_liquidity() -> Weight {
		(166_749_000_u64)
			.saturating_add(DbWeight::get().reads(7_u64))
			.saturating_add(DbWeight::get().writes(5_u64))
	}
	fn withdraw_liquidity() -> Weight {
		(286_666_000_u64)
			.saturating_add(DbWeight::get().reads(8_u64))
			.saturating_add(DbWeight::get().writes(5_u64))
	}
	fn set_identity() -> Weight {
		(120_932_000_u64)
			.saturating_add(DbWeight::get().reads(7_u64))
			.saturating_add(DbWeight::get().writes(4_u64))
	}
	fn verify_identity() -> Weight {
		(68_936_000_u64)
			.saturating_add(DbWeight::get().reads(5_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
	fn clear_identity() -> Weight {
		(134_464_000_u64)
			.saturating_add(DbWeight::get().reads(7_u64))
			.saturating_add(DbWeight::get().writes(4_u64))
	}
	fn transfer_liquidity_pool() -> Weight {
		(86_430_000_u64)
			.saturating_add(DbWeight::get().reads(6_u64))
			.saturating_add(DbWeight::get().writes(3_u64))
	}
}
