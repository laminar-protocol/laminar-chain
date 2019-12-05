//! Mocks for the synthetic-protocol module.

#![cfg(test)]

use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use frame_system as system;
use primitives::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use orml_currencies::Currency;

use module_primitives::{BalancePriceConverter, LiquidityPoolId};

use super::*;

pub use module_primitives::{Balance, CurrencyId};

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod synthetic_protocol {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		orml_tokens<T>, orml_currencies<T>,
		synthetic_tokens<T>, synthetic_protocol<T>,
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

pub type AccountId = u32;
impl frame_system::Trait for Runtime {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}
pub type System = system::Module<Runtime>;

type Amount = i128;
impl orml_tokens::Trait for Runtime {
	type Event = TestEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::FLOW;
}

type NativeCurrency = Currency<Runtime, GetNativeCurrencyId>;

impl orml_currencies::Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = NativeCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
}

parameter_types! {
	pub const GetCollateralCurrencyId: CurrencyId = CurrencyId::AUSD;
	pub const GetSyntheticCurrencyId: CurrencyId = CurrencyId::FEUR;
}

pub type CollateralCurrency = orml_currencies::Currency<Runtime, GetCollateralCurrencyId>;
pub type SyntheticCurrency = orml_currencies::Currency<Runtime, GetSyntheticCurrencyId>;

impl synthetic_tokens::Trait for Runtime {
	type Event = TestEvent;
	type CurrencyId = CurrencyId;
	type Balance = Balance;
	type LiquidityPoolId = LiquidityPoolId;
}
pub type SyntheticTokens = synthetic_tokens::Module<Runtime>;

/// Mock prices module, implements `PriceProvider`, with configurable prices to test different cases.
pub mod mock_prices {
	use frame_support::{decl_module, decl_storage, Parameter, StorageMap};
	// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
	// would cause compiling error in `decl_module!` and `construct_runtime!`
	// #3295 https://github.com/paritytech/substrate/issues/3295
	use super::Price;
	use orml_traits::PriceProvider;
	use sp_runtime::traits::{MaybeSerializeDeserialize, Member};

	pub trait Trait: frame_system::Trait {
		type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize;
	}

	decl_storage! {
		trait Store for Module<T: Trait> as MockPrices {
			pub Prices get(fn prices): map T::CurrencyId => Option<Price>;
		}

		add_extra_genesis {
			config(prices): Vec<(T::CurrencyId, Price)>;
			build(|config: &GenesisConfig<T>| {
				config.prices.iter().for_each(|(currency_id, price)| {
					<Prices<T>>::insert(currency_id, price);
				})
			})
		}
	}

	decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
	}

	impl<T: Trait> Module<T> {
		pub fn set_mock_price(currency_id: T::CurrencyId, price: Option<Price>) {
			if let Some(p) = price {
				<Prices<T>>::insert(currency_id, p);
			} else {
				<Prices<T>>::remove(currency_id);
			}
		}
	}

	impl<T: Trait> PriceProvider<T::CurrencyId, Price> for Module<T> {
		fn get_price(base: T::CurrencyId, quote: T::CurrencyId) -> Option<Price> {
			let base_price = Self::prices(base)?;
			let quote_price = Self::prices(quote)?;

			quote_price.checked_div(&base_price)
		}
	}
}

impl mock_prices::Trait for Runtime {
	type CurrencyId = CurrencyId;
}
pub type MockPrices = mock_prices::Module<Runtime>;

/// Mock liquidity pool module, implements liquidity pool related traits, with configurable additional collateral
/// ratio and ask/bid spread, to test different cases.
pub mod mock_liquidity_pool {
	use frame_support::{decl_module, decl_storage, Parameter};
	// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
	// would cause compiling error in `decl_module!` and `construct_runtime!`
	// #3295 https://github.com/paritytech/substrate/issues/3295
	use super::{AccountId, Permill};
	use orml_traits::BasicCurrency;
	use sp_runtime::traits::{MaybeSerializeDeserialize, Member, SimpleArithmetic};
	use traits::{LiquidityPoolBaseTypes, LiquidityPoolsConfig, LiquidityPoolsCurrency};

	pub trait Trait: frame_system::Trait {
		type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize;
		type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
		type CollateralCurrency: BasicCurrency<AccountId, Balance = Self::Balance>;
	}

	decl_storage! {
		trait Store for Module<T: Trait> as MockLiquidityPools {
			pub AdditionalCollateralRatio get(fn additional_collateral_ratio) config(): Permill;
			pub Spread get(fn spread) config(): Permill;
		}
	}

	decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
	}

	impl<T: Trait> Module<T> {}

	impl<T: Trait> LiquidityPoolBaseTypes for Module<T> {
		type LiquidityPoolId = AccountId;
		type CurrencyId = T::CurrencyId;
	}

	impl<T: Trait> LiquidityPoolsConfig for Module<T> {
		fn get_bid_spread(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
			Self::spread()
		}

		fn get_ask_spread(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
			Self::spread()
		}

		fn get_additional_collateral_ratio(_pool_id: Self::LiquidityPoolId, _currency_id: Self::CurrencyId) -> Permill {
			Self::additional_collateral_ratio()
		}
	}

	impl<T: Trait> LiquidityPoolsCurrency<AccountId> for Module<T> {
		type Balance = T::Balance;
		type Error = &'static str;

		fn balance(pool_id: Self::LiquidityPoolId) -> Self::Balance {
			T::CollateralCurrency::balance(&pool_id)
		}

		fn deposit(from: &AccountId, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> Result<(), Self::Error> {
			T::CollateralCurrency::transfer(from, &pool_id, amount).map_err(|e| e.into())
		}

		fn withdraw(to: &AccountId, pool_id: Self::LiquidityPoolId, amount: Self::Balance) -> Result<(), Self::Error> {
			T::CollateralCurrency::transfer(&pool_id, to, amount).map_err(|e| e.into())
		}
	}
}

impl mock_liquidity_pool::Trait for Runtime {
	type CurrencyId = CurrencyId;
	type Balance = Balance;
	type CollateralCurrency = CollateralCurrency;
}
pub type MockLiquidityPools = mock_liquidity_pool::Module<Runtime>;

impl Trait for Runtime {
	type Event = TestEvent;
	type MultiCurrency = orml_currencies::Module<Runtime>;
	type CollateralCurrency = CollateralCurrency;
	type GetCollateralCurrencyId = GetCollateralCurrencyId;
	type PriceProvider = MockPrices;
	type LiquidityPoolsConfig = MockLiquidityPools;
	type LiquidityPoolsCurrency = MockLiquidityPools;
	type BalanceToPrice = BalancePriceConverter;
	type PriceToBalance = BalancePriceConverter;
}
pub type SyntheticProtocol = Module<Runtime>;

pub const ALICE: AccountId = 0;
pub const BOB: AccountId = 1;
pub fn origin_of(account_id: AccountId) -> Origin {
	Origin::signed(account_id)
}

pub const MOCK_POOL: LiquidityPoolId = 100;
pub const ANOTHER_MOCK_POOL: LiquidityPoolId = 101;

pub struct ExtBuilder {
	currency_id: CurrencyId,
	endowed_accounts: Vec<AccountId>,
	initial_balance: Balance,
	prices: Vec<(CurrencyId, Price)>,
	spread: Permill,
	additional_collateral_ratio: Permill,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			currency_id: CurrencyId::AUSD,
			endowed_accounts: vec![0],
			initial_balance: 0,
			// collateral price set to `1` for calculation simplicity.
			prices: vec![(CurrencyId::AUSD, FixedU128::from_rational(1, 1))],
			spread: Permill::zero(),
			additional_collateral_ratio: Permill::zero(),
		}
	}
}

pub const ONE_MILL: Balance = 1000_000;
impl ExtBuilder {
	pub fn balances(mut self, account_ids: Vec<AccountId>, initial_balance: Balance) -> Self {
		self.endowed_accounts = account_ids;
		self.initial_balance = initial_balance;
		self
	}

	// one_million is big enough for testing, considering spread is 0.5% on average, and small enough
	// to do math verification by hand.
	pub fn one_million_for_alice_n_mock_pool(self) -> Self {
		self.balances(vec![ALICE, MOCK_POOL], ONE_MILL)
	}

	pub fn synthetic_price(mut self, price: Price) -> Self {
		self.prices.push((CurrencyId::FEUR, price));
		self
	}

	/// set synthetic price to `3`
	pub fn synthetic_price_three(self) -> Self {
		self.synthetic_price(Price::from_rational(3, 1))
	}

	pub fn spread(mut self, spread: Permill) -> Self {
		self.spread = spread;
		self
	}

	pub fn one_percent_spread(self) -> Self {
		self.spread(Permill::from_percent(1))
	}

	pub fn additional_collateral_ratio(mut self, ratio: Permill) -> Self {
		self.additional_collateral_ratio = ratio;
		self
	}

	pub fn ten_percent_additional_collateral_ratio(self) -> Self {
		self.additional_collateral_ratio(Permill::from_percent(10))
	}

	pub fn build(self) -> runtime_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			tokens: vec![self.currency_id],
			initial_balance: self.initial_balance,
			endowed_accounts: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		mock_prices::GenesisConfig::<Runtime> { prices: self.prices }
			.assimilate_storage(&mut t)
			.unwrap();

		mock_liquidity_pool::GenesisConfig {
			spread: self.spread,
			additional_collateral_ratio: self.additional_collateral_ratio,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
