use crate::{AccountId, Balance, Currencies, CurrencyId, MinimumCount, Oracle, Price, Runtime, DOLLARS};

use frame_support::{assert_ok, traits::OnFinalize};
use orml_traits::{MultiCurrency, MultiCurrencyExtended, PriceProvider};
use sp_runtime::traits::{SaturatedConversion, StaticLookup};

pub fn lookup_of_account(who: AccountId) -> <<Runtime as frame_system::Trait>::Lookup as StaticLookup>::Source {
	<Runtime as frame_system::Trait>::Lookup::unlookup(who)
}

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) {
	assert_ok!(<Currencies as MultiCurrencyExtended<_>>::update_balance(
		currency_id,
		&who,
		balance.saturated_into()
	));
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

type Prices = orml_prices::DefaultPriceProvider<CurrencyId, Oracle>;

pub fn set_price(prices: Vec<(CurrencyId, Price)>) {
	Oracle::on_finalize(0);
	for i in 1..=MinimumCount::get() {
		assert_ok!(Oracle::feed_values(
			<Runtime as frame_system::Trait>::Origin::NONE,
			prices.clone(),
			i, // i as u32,
			Default::default()
		));
	}
	Prices::get_price(CurrencyId::FEUR, CurrencyId::AUSD);
}
