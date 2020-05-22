use crate::{AccountId, Balance, Currencies, CurrencyId, Runtime, DOLLARS};

use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use sp_runtime::traits::{SaturatedConversion, StaticLookup};

pub fn lookup_of_account(who: AccountId) -> <<Runtime as frame_system::Trait>::Lookup as StaticLookup>::Source {
	<Runtime as frame_system::Trait>::Lookup::unlookup(who)
}

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) {
	let _ = <Currencies as MultiCurrencyExtended<_>>::update_balance(currency_id, &who, balance.saturated_into());
	assert_eq!(
		<Currencies as MultiCurrency<_>>::free_balance(currency_id, who),
		balance
	);
}

pub fn set_ausd_balance(who: &AccountId, balance: Balance) {
	set_balance(CurrencyId::AUSD, who, balance)
}

pub fn dollars<T: Into<u128>>(d: T) -> Balance {
	DOLLARS.saturating_mul(d.into())
}
