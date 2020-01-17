/// tests for this module

#[cfg(test)]

mod tests {
	pub use flowchain_runtime::{AccountId, CurrencyId, LiquidityPoolId, Runtime};
	use frame_support::assert_ok;
	pub use module_primitives::{Balance, Leverage};
	pub use orml_prices::Price;
	use orml_traits::{BasicCurrency, MultiCurrency};
	pub use sp_runtime::{traits::Zero, DispatchResult, Perbill, Permill};

	pub fn origin_of(account_id: AccountId) -> <Runtime as system::Trait>::Origin {
		<Runtime as system::Trait>::Origin::signed(account_id)
	}

	pub type ModuleProtocol = synthetic_protocol::Module<Runtime>;
	pub type ModuleTokens = synthetic_tokens::Module<Runtime>;
	pub type ModuleOracle = orml_oracle::Module<Runtime>;
	pub type ModulePrices = orml_prices::Module<Runtime>;
	pub type ModuleLiquidityPools = liquidity_pools::Module<Runtime>;

	const LIQUIDITY_POOL_ID: LiquidityPoolId = 0;

	const ORACLE1: [u8; 32] = [0u8; 32];
	const ORACLE2: [u8; 32] = [1u8; 32];
	const ORACLE3: [u8; 32] = [2u8; 32];

	const POOL: [u8; 32] = [3u8; 32];
	const ALICE: [u8; 32] = [4u8; 32];
	const BOB: [u8; 32] = [5u8; 32];

	pub struct ExtBuilder {
		endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
		prices: Vec<(CurrencyId, Price)>,
		spread: Permill,
		additional_collateral_ratio: Permill,
	}

	impl Default for ExtBuilder {
		fn default() -> Self {
			Self {
				endowed_accounts: vec![],
				// collateral price set to `1` for calculation simplicity.
				prices: vec![],
				spread: Permill::zero(),
				additional_collateral_ratio: Permill::zero(),
			}
		}
	}

	impl ExtBuilder {
		pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
			self.endowed_accounts = endowed_accounts;
			self
		}

		pub fn build(self) -> sp_io::TestExternalities {
			let mut t = system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

			orml_tokens::GenesisConfig::<Runtime> {
				endowed_accounts: self.endowed_accounts,
			}
			.assimilate_storage(&mut t)
			.unwrap();

			pallet_collective::GenesisConfig::<Runtime, _> {
				members: vec![
					AccountId::from(ORACLE1),
					AccountId::from(ORACLE2),
					AccountId::from(ORACLE3),
				],
				phantom: Default::default(),
			}
			.assimilate_storage(&mut t)
			.unwrap();

			t.into()
		}
	}

	pub fn create_pool() -> DispatchResult {
		ModuleLiquidityPools::create_pool(origin_of(AccountId::from(POOL)))
	}

	pub fn deposit_liquidity(amount: Balance) -> DispatchResult {
		ModuleLiquidityPools::deposit_liquidity(origin_of(AccountId::from(POOL)), LIQUIDITY_POOL_ID, amount)
	}

	// additional_collateral_ratio
	pub fn set_additional_collateral_ratio(permill: Permill) -> DispatchResult {
		ModuleLiquidityPools::set_additional_collateral_ratio(
			origin_of(AccountId::from(POOL)),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			Some(permill),
		)
	}

	// spread
	pub fn set_spread(permill: Permill) -> DispatchResult {
		ModuleLiquidityPools::set_spread(
			origin_of(AccountId::from(POOL)),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			permill,
			permill,
		)
	}

	fn set_oracle_price(prices: Vec<(CurrencyId, Price)>) -> DispatchResult {
		prices.iter().for_each(|(c, p)| {
			assert_ok!(ModuleOracle::feed_value(origin_of(AccountId::from(ORACLE1)), *c, *p));
			assert_ok!(ModuleOracle::feed_value(origin_of(AccountId::from(ORACLE2)), *c, *p));
			assert_ok!(ModuleOracle::feed_value(origin_of(AccountId::from(ORACLE3)), *c, *p));
		});
		Ok(())
	}

	fn buy(who: &AccountId, amount: Balance) -> DispatchResult {
		ModuleProtocol::mint(
			origin_of(who.clone()),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			amount,
			Permill::from_percent(10),
		)
	}

	fn sell(who: &AccountId, amount: Balance) -> DispatchResult {
		ModuleProtocol::redeem(
			origin_of(who.clone()),
			LIQUIDITY_POOL_ID,
			CurrencyId::FEUR,
			amount,
			Permill::from_percent(10),
		)
	}

	fn collateral_balance(who: &AccountId) -> Balance {
		<Runtime as synthetic_protocol::Trait>::CollateralCurrency::balance(who)
	}

	fn synthetic_balance(who: &AccountId) -> Balance {
		<Runtime as synthetic_protocol::Trait>::MultiCurrency::balance(CurrencyId::FEUR, &who)
	}

	fn liquidity() -> Balance {
		ModuleLiquidityPools::balances(LIQUIDITY_POOL_ID)
	}

	#[test]
	fn test_can_buy_and_sell() {
		ExtBuilder::default()
			.balances(vec![
				(AccountId::from(POOL), CurrencyId::AUSD, 10_000),
				(AccountId::from(ALICE), CurrencyId::AUSD, 10_000),
			])
			.build()
			.execute_with(|| {
				assert_ok!(create_pool());
				assert_ok!(deposit_liquidity(10_000));
				assert_ok!(set_additional_collateral_ratio(Permill::from_percent(10)));
				assert_ok!(set_spread(Permill::from_percent(1)));
				assert_ok!(set_oracle_price(vec![
					(CurrencyId::AUSD, Price::from_rational(1, 1)),
					(CurrencyId::FEUR, Price::from_rational(3, 1))
				]));

				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 10_000);
				assert_eq!(collateral_balance(&AccountId::from(POOL)), 0);
				assert_eq!(liquidity(), 10_000);
				// ExistentialDeposit = 500, so the first time amount >= 500;
				assert_ok!(buy(&AccountId::from(ALICE), 1001));
				assert_eq!(collateral_balance(&AccountId::from(ALICE)), 8999);

				//TODO:
				assert_eq!(liquidity(), 9912);
				//TODO:
				assert_eq!(synthetic_balance(&AccountId::from(ALICE)), 0);
				//TODO:
				assert_eq!(synthetic_balance(&AccountId::from(POOL)), 0);
				//		balance(iUsd, fToken.address, dollar(11000)),
				//		balance(iUsd, liquidityPool.address, dollar(99010)),
				//
				assert_ok!(sell(&AccountId::from(ALICE), 1_000));
				//		balance(fToken, alice, 0),
				//		balance(usd, alice, dollar(9998)),
				//		balance(iUsd, fToken.address, 0),
				//		balance(iUsd, liquidityPool.address, dollar(100020)),
				//});
			});
	}
}
