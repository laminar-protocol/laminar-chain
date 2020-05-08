//! Utilities for margin protocol benchmarking

#![cfg(feature = "runtime-benchmarks")]

use super::*;

impl<T: Trait> Module<T> {
	pub fn transfer_usd(from: &T::AccountId, to: &T::AccountId, amount: Balance) -> DispatchResult {
		T::MultiCurrency::transfer(CurrencyId::AUSD, from, to, amount)
	}
}
