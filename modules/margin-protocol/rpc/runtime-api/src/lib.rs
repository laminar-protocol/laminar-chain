//! Runtime API definition for margin protocol module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

sp_api::decl_runtime_apis! {
	pub trait MarginProtocolApi<AccountId, Fixed128> where
		AccountId: Codec,
		Fixed128: Codec,
	{
		fn equity_of_trader(who: AccountId) -> Option<Fixed128>;
		fn margin_level(who: AccountId) -> Option<Fixed128>;
		fn unrealized_pl_of_trader(who: AccountId) -> Option<Fixed128>;
	}
}
