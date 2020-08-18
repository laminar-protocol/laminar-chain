use crate::{AccountId, Balance, Currencies, CurrencyId, MinimumCount, Oracle, Price, Runtime, DOLLARS};

use frame_support::traits::OnFinalize;
use orml_traits::{MultiCurrencyExtended, PriceProvider};
use sp_runtime::{
	traits::{SaturatedConversion, StaticLookup},
	DispatchResult,
};

pub fn lookup_of_account(who: AccountId) -> <<Runtime as frame_system::Trait>::Lookup as StaticLookup>::Source {
	<Runtime as frame_system::Trait>::Lookup::unlookup(who)
}

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) -> DispatchResult {
	<Currencies as MultiCurrencyExtended<_>>::update_balance(currency_id, &who, balance.saturated_into())
}

pub fn set_ausd_balance(who: &AccountId, balance: Balance) -> DispatchResult {
	set_balance(CurrencyId::AUSD, who, balance)
}

pub fn dollars<T: Into<u128>>(d: T) -> Balance {
	DOLLARS.saturating_mul(d.into())
}

type Prices = orml_traits::DefaultPriceProvider<CurrencyId, Oracle>;

pub fn set_price(prices: sp_std::vec::Vec<(CurrencyId, Price)>) -> DispatchResult {
	Oracle::on_finalize(0);
	for i in 0..MinimumCount::get() {
		Oracle::feed_values(
			<Runtime as frame_system::Trait>::Origin::root(),
			prices.clone(),
			i,
			0,
			Default::default(),
		)?;
	}
	Prices::get_price(CurrencyId::FEUR, CurrencyId::AUSD);

	Ok(())
}
