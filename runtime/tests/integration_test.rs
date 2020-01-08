/// tests for this module
#[cfg(test)]
mod tests {
	extern crate flowchain_runtime;

	use flowchain_runtime::Runtime;
	use frame_support::assert_ok;
	pub use sp_runtime::Permill;

	// This function basically just builds a genesis storage key/value store according to
	// our desired mockup.
	fn new_test_ext() -> sp_io::TestExternalities {
		system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap()
			.into()
	}

	type AccountId<T> = <T as system::Trait>::AccountId;

	pub type ModuleProtocol = synthetic_protocol::Module<Runtime>;
	pub type ModuleLiquidityPools = liquidity_pools::Module<Runtime>;

	#[test]
	fn test_liquidity_pools() {
		let ALICE: AccountId<Runtime> = AccountId::<Runtime>::from([0u8; 32]);
		//let ALICE :<flowchain_runtime::Runtime as system::Trait>::AccountId  = <flowchain_runtime::Runtime as system::Trait>::AccountId::from([0u8; 32]);

		new_test_ext().execute_with(|| {
			//<flowchain_runtime::Runtime as liquidity_pools::Trait>::create_pool(<flowchain_runtime::Runtime as system::Trait>::Origin::signed(ALICE));
			assert_ok!(ModuleLiquidityPools::create_pool(
				<Runtime as system::Trait>::Origin::signed(ALICE.clone())
			));
			assert_eq!(ModuleLiquidityPools::is_owner(0, &ALICE.clone()), true);
			assert_eq!(ModuleLiquidityPools::is_owner(1, &ALICE.clone()), false);
		});
	}

	#[test]
	fn test_protocol() {
		pub const MOCK_POOL: flowchain_runtime::LiquidityPoolId = 100;
		pub const ANOTHER_MOCK_POOL: flowchain_runtime::LiquidityPoolId = 101;

		let ALICE: AccountId<Runtime> = AccountId::<Runtime>::from([0u8; 32]);

		new_test_ext().execute_with(|| {
			// buy
			ModuleProtocol::mint(
				<Runtime as system::Trait>::Origin::signed(ALICE.clone()),
				MOCK_POOL,
				flowchain_runtime::CurrencyId::FEUR,
				1,
				Permill::from_percent(10),
			);
			// balance

			// sell
			ModuleProtocol::redeem(
				<Runtime as system::Trait>::Origin::signed(ALICE.clone()),
				MOCK_POOL,
				flowchain_runtime::CurrencyId::FEUR,
				1,
				Permill::from_percent(10),
			);
			// balance
		});
	}
}
