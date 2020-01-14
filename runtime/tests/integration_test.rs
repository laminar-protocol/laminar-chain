/// tests for this module
#[cfg(test)]
mod tests {
	extern crate flowchain_runtime;

	use flowchain_runtime::Runtime;
	use frame_support::{assert_noop, assert_ok};
	use module_primitives::Balance;
	use orml_prices::Price;
	use orml_traits::BasicCurrency;
	use sp_runtime::{DispatchResult, Permill};

	static ORACLE_ID1: &'static [u8; 32] = &[0u8; 32];
	static ORACLE_ID2: &'static [u8; 32] = &[1u8; 32];
	static ORACLE_ID3: &'static [u8; 32] = &[2u8; 32];
	static POOL_ID: &'static [u8; 32] = &[3u8; 32];
	static PROTOCOL_ID: &'static [u8; 32] = &[4u8; 32];
	static ALICE_ID: &'static [u8; 32] = &[5u8; 32];
	static BOB_ID: &'static [u8; 32] = &[6u8; 32];

	const LIQUIDITY_POOL_ID: flowchain_runtime::LiquidityPoolId = 0;
	const LIQUIDITY_NEXT_POOL_ID: flowchain_runtime::LiquidityPoolId = 1;

	type AccountIdOf<T> = <T as system::Trait>::AccountId;

	pub type ModuleProtocol = synthetic_protocol::Module<Runtime>;
	pub type ModuleTokens = synthetic_tokens::Module<Runtime>;
	pub type ModuleOracle = orml_oracle::Module<Runtime>;
	pub type ModulePrices = orml_prices::Module<Runtime>;
	pub type ModuleLiquidityPools = liquidity_pools::Module<Runtime>;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap()
			.into();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: vec![
				(
					AccountIdOf::<Runtime>::from(*ALICE_ID),
					flowchain_runtime::CurrencyId::AUSD,
					10_000,
				),
				(
					AccountIdOf::<Runtime>::from(*BOB_ID),
					flowchain_runtime::CurrencyId::AUSD,
					10_000,
				),
				(
					AccountIdOf::<Runtime>::from(*POOL_ID),
					flowchain_runtime::CurrencyId::AUSD,
					20_000,
				),
				//(AccountIdOf::<Runtime>::from(*POOL_ID), flowchain_runtime::CurrencyId::FEUR, 100_000),
				//(AccountIdOf::<Runtime>::from(*PROTOCOL_ID), flowchain_runtime::CurrencyId::AUSD, 20_000),
				(
					AccountIdOf::<Runtime>::from(*PROTOCOL_ID),
					flowchain_runtime::CurrencyId::FEUR,
					20_000,
				),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_collective::GenesisConfig::<Runtime, _> {
			members: vec![
				AccountIdOf::<Runtime>::from(*ORACLE_ID1),
				AccountIdOf::<Runtime>::from(*ORACLE_ID2),
				AccountIdOf::<Runtime>::from(*ORACLE_ID3),
			],
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}

	fn setup() {
		let pool: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*POOL_ID);
		let oracle1: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*ORACLE_ID1);
		let oracle2: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*ORACLE_ID2);
		let oracle3: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*ORACLE_ID3);
		let protocol: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*PROTOCOL_ID);

		assert_ok!(ModuleLiquidityPools::create_pool(
			<Runtime as system::Trait>::Origin::signed(pool.clone())
		));

		assert_ok!(ModuleOracle::feed_value(
			<Runtime as system::Trait>::Origin::signed(oracle1.clone()),
			flowchain_runtime::CurrencyId::AUSD,
			Price::from_parts(1)
		));
		assert_ok!(ModuleOracle::feed_value(
			<Runtime as system::Trait>::Origin::signed(oracle2.clone()),
			flowchain_runtime::CurrencyId::AUSD,
			Price::from_parts(1)
		));
		assert_ok!(ModuleOracle::feed_value(
			<Runtime as system::Trait>::Origin::signed(oracle3.clone()),
			flowchain_runtime::CurrencyId::AUSD,
			Price::from_parts(1)
		));

		assert_ok!(ModuleOracle::feed_value(
			<Runtime as system::Trait>::Origin::signed(oracle1.clone()),
			flowchain_runtime::CurrencyId::FEUR,
			Price::from_parts(1)
		));
		assert_ok!(ModuleOracle::feed_value(
			<Runtime as system::Trait>::Origin::signed(oracle2.clone()),
			flowchain_runtime::CurrencyId::FEUR,
			Price::from_parts(1)
		));
		assert_ok!(ModuleOracle::feed_value(
			<Runtime as system::Trait>::Origin::signed(oracle3.clone()),
			flowchain_runtime::CurrencyId::FEUR,
			Price::from_parts(1)
		));

		assert_ok!(ModuleLiquidityPools::deposit_liquidity(
			<Runtime as system::Trait>::Origin::signed(pool.clone()),
			LIQUIDITY_POOL_ID,
			5_000
		));
		assert_ok!(ModuleLiquidityPools::set_additional_collateral_ratio(
			<Runtime as system::Trait>::Origin::signed(pool.clone()),
			LIQUIDITY_POOL_ID,
			flowchain_runtime::CurrencyId::FEUR,
			Some(Permill::from_percent(50))
		));
		assert_ok!(ModuleLiquidityPools::set_spread(
			<Runtime as system::Trait>::Origin::signed(pool.clone()),
			LIQUIDITY_POOL_ID,
			flowchain_runtime::CurrencyId::FEUR,
			Permill::from_percent(100),
			Permill::from_percent(100)
		));
		//assert_ok!(ModuleLiquidityPools::set_spread(<Runtime as system::Trait>::Origin::signed(pool.clone()), LIQUIDITY_POOL_ID, flowchain_runtime::CurrencyId::FEUR, Permill::zero(), Permill::zero()));

		//assert_ok!(ModuleLiquidityPools::deposit_liquidity(<Runtime as system::Trait>::Origin::signed(protocol.clone()), LIQUIDITY_POOL_ID, 20_000));
		(ModuleTokens::add_position(LIQUIDITY_POOL_ID, flowchain_runtime::CurrencyId::FEUR, 5_000, 5_000));
		assert_ok!(ModuleProtocol::liquidate(
			<Runtime as system::Trait>::Origin::signed(protocol.clone()),
			LIQUIDITY_POOL_ID,
			flowchain_runtime::CurrencyId::FEUR,
			5_000
		));
	}

	#[test]
	fn test_liquidity_pools() {
		let pool: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*POOL_ID);

		new_test_ext().execute_with(|| {
			assert_ok!(ModuleLiquidityPools::create_pool(
				<Runtime as system::Trait>::Origin::signed(pool.clone())
			));
			assert_eq!(ModuleLiquidityPools::is_owner(LIQUIDITY_POOL_ID, &pool.clone()), true);
			assert_eq!(
				ModuleLiquidityPools::is_owner(LIQUIDITY_NEXT_POOL_ID, &pool.clone()),
				false
			);

			assert_ok!(ModuleLiquidityPools::set_spread(
				<Runtime as system::Trait>::Origin::signed(pool.clone()),
				LIQUIDITY_POOL_ID,
				flowchain_runtime::CurrencyId::AUSD,
				Permill::from_percent(10),
				Permill::from_percent(10)
			));
		});
	}

	fn buy(who: &AccountIdOf<Runtime>, amount: Balance) -> DispatchResult {
		ModuleProtocol::mint(
			<Runtime as system::Trait>::Origin::signed(who.clone()),
			LIQUIDITY_POOL_ID,
			flowchain_runtime::CurrencyId::FEUR,
			amount,
			Permill::from_percent(100),
		)
	}

	fn sell(who: &AccountIdOf<Runtime>, amount: Balance) -> DispatchResult {
		ModuleProtocol::redeem(
			<Runtime as system::Trait>::Origin::signed(who.clone()),
			LIQUIDITY_POOL_ID,
			flowchain_runtime::CurrencyId::FEUR,
			amount,
			Permill::from_percent(100),
		)
	}

	fn balance(who: &AccountIdOf<Runtime>) -> Balance {
		<Runtime as synthetic_protocol::Trait>::CollateralCurrency::balance(&who)
	}

	//fn set_price(who: &AccountIdOf<Runtime>, price :u128) -> DispatchResult {
	//	ModuleOracle::feed_value(<Runtime as system::Trait>::Origin::signed(who.clone()), flowchain_runtime::CurrencyId::AUSD, Price::from_parts(price));
	//	ModuleOracle::feed_value(<Runtime as system::Trait>::Origin::signed(who.clone()), flowchain_runtime::CurrencyId::FEUR, Price::from_parts(price));
	//}

	#[test]
	fn test_can_buy_and_sell() {
		let pool: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*POOL_ID);
		let alice: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*ALICE_ID);

		new_test_ext().execute_with(|| {
			setup();
			assert_eq!(balance(&alice), 10000);
			assert_ok!(buy(&pool, 1001));
			//assert_eq!(balance(alice), 1000);
			//		balance(usd, alice, dollar(8999)),
			//		balance(iUsd, fToken.address, dollar(11000)),
			//		balance(iUsd, liquidityPool.address, dollar(99010)),
			//
			//		sell(alice, dollar(1000)),
			//		balance(fToken, alice, 0),
			//		balance(usd, alice, dollar(9998)),
			//		balance(iUsd, fToken.address, 0),
			//		balance(iUsd, liquidityPool.address, dollar(100020)),
		});
	}

	//#[test]
	//fn test_can_take_profit() {
	//	let pool : AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*POOL_ID);
	//	let alice: AccountIdOf<Runtime> = AccountIdOf::<Runtime>::from(*ALICE_ID);

	//	new_test_ext().execute_with(|| {
	//		setup();
	//		buy(alice, dollar(1001)),
	//		balance(fToken, alice, dollar(1000)),
	//		balance(usd, alice, dollar(8999)),
	//		balance(iUsd, fToken.address, dollar(11000)),
	//		balance(iUsd, liquidityPool.address, dollar(99010)),
	//		setPrice(105),

	//		sell(alice, dollar(1000)),
	//		balance(fToken, alice, 0),
	//		balance(usd, alice, '10047950000000000000000'),
	//		balance(iUsd, fToken.address, 0),
	//		balance(iUsd, liquidityPool.address, '99520500000000000000000'),
	//	});
	//}

	//#[test]
	//fn test_can_stop_lost() {
	//	new_test_ext().execute_with(|| {
	//		buy(alice, dollar(1001)),
	//		balance(fToken, alice, dollar(1000)),
	//		balance(usd, alice, dollar(8999)),
	//		balance(iUsd, fToken.address, dollar(11000)),
	//		balance(iUsd, liquidityPool.address, dollar(99010)),
	//		setPrice(95),

	//		sell(alice, dollar(1000)),
	//		balance(fToken, alice, 0),
	//		balance(usd, alice, '9948050000000000000000'),
	//		balance(iUsd, fToken.address, 0),
	//		balance(iUsd, liquidityPool.address, '100519500000000000000000'),
	//	});
	//}
}
