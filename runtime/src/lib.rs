//! The Laminar runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use pallet_grandpa::fg_primitives;
use pallet_grandpa::AuthorityList as GrandpaAuthorityList;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::u32_trait::{_1, _2};
use sp_core::OpaqueMetadata;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT, ConvertInto, IdentifyAccount, StaticLookup, Verify};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys, transaction_validity::TransactionValidity, ApplyExtrinsicResult,
	MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use module_primitives::{CurrencyId, LiquidityPoolId};
use orml_currencies::BasicCurrencyAdapter;
use orml_oracle::OperatorProvider;

use module_primitives::{BalancePriceConverter, Price};
use traits::LiquidityPoolManager;

// A few exports that help ease life for downstream crates.
pub use frame_support::{construct_runtime, parameter_types, traits::Randomness, weights::Weight, StorageValue};
pub use module_primitives::Balance;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{traits::Zero, Perbill, Permill};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them, but you
/// never know...
pub type AccountIndex = u32;

/// Signed version of Balance
pub type Amount = i128;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core datastructures.
pub mod opaque {
	use super::*;

	pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

	/// Opaque block header type.
	pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = generic::Block<Header, UncheckedExtrinsic>;
	/// Opaque block identifier type.
	pub type BlockId = generic::BlockId<Block>;

	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
		}
	}
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("laminar-chain"),
	impl_name: create_runtime_str!("laminar-chain"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
};

pub const MILLISECS_PER_BLOCK: u64 = 4000;

pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

/// The version infromation used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const MaximumBlockWeight: Weight = 1_000_000;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
}

impl system::Trait for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = Indices;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Maximum weight of each block.
	type MaximumBlockWeight = MaximumBlockWeight;
	/// Maximum size of all encoded transactions (in bytes) that are allowed in one block.
	type MaximumBlockLength = MaximumBlockLength;
	/// Portion of the block weight that is available to all normal transactions.
	type AvailableBlockRatio = AvailableBlockRatio;
	/// Version of the runtime.
	type Version = Version;
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type ModuleToIndex = ModuleToIndex;
}

impl pallet_aura::Trait for Runtime {
	type AuthorityId = AuraId;
}

impl pallet_grandpa::Trait for Runtime {
	type Event = Event;
}

impl pallet_indices::Trait for Runtime {
	/// The type for recording indexing into the account enumeration. If this ever overflows, there
	/// will be problems!
	type AccountIndex = AccountIndex;
	/// Use the standard means of resolving an index hint from an id.
	type ResolveHint = pallet_indices::SimpleResolveHint<Self::AccountId, Self::AccountIndex>;
	/// Determine whether an account is dead.
	type IsDeadAccount = Balances;
	/// The ubiquitous event type.
	type Event = Event;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = MinimumPeriod;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
}

impl pallet_balances::Trait for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// What to do if an account's free balance gets zeroed.
	type OnFreeBalanceZero = ();
	/// What to do if a new account is created.
	type OnNewAccount = Indices;
	type OnReapAccount = System;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type TransferPayment = ();
	type ExistentialDeposit = ExistentialDeposit;
	type TransferFee = TransferFee;
	type CreationFee = CreationFee;
}

parameter_types! {
	pub const TransactionBaseFee: Balance = 0;
	pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Trait for Runtime {
	type Currency = pallet_balances::Module<Runtime>;
	type OnTransactionPayment = ();
	type TransactionBaseFee = TransactionBaseFee;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = ConvertInto;
	type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Trait for Runtime {
	type Event = Event;
	type Proposal = Call;
}

type OperatorCollectiveInstance = pallet_collective::Instance1;
impl pallet_collective::Trait<OperatorCollectiveInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
}

type OperatorMembershipInstance = pallet_membership::Instance1;
impl pallet_membership::Trait<OperatorMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, OperatorCollectiveInstance>;
	type RemoveOrigin = pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, OperatorCollectiveInstance>;
	type SwapOrigin = pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, OperatorCollectiveInstance>;
	type ResetOrigin = pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, OperatorCollectiveInstance>;
	type MembershipInitialized = OperatorCollective;
	type MembershipChanged = OperatorCollective;
}

pub struct OperatorCollectiveProvider;
impl OperatorProvider<AccountId> for OperatorCollectiveProvider {
	fn can_feed_data(who: &AccountId) -> bool {
		OperatorCollective::is_member(who)
	}

	fn operators() -> Vec<AccountId> {
		OperatorCollective::members()
	}
}

// TODO: set this
parameter_types! {
	pub const MinimumCount: u32 = 3;
	pub const ExpiresIn: u32 = 600;
}

impl orml_oracle::Trait for Runtime {
	type Event = Event;
	type OnNewData = (); // TODO: update this
	type OperatorProvider = OperatorCollectiveProvider;
	type CombineData = orml_oracle::DefaultCombineData<Runtime, MinimumCount, ExpiresIn>;
	type Time = Timestamp;
	type OracleKey = CurrencyId;
	type OracleValue = Price;
}

impl orml_tokens::Trait for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type ExistentialDeposit = ExistentialDeposit;
	type DustRemoval = ();
}

parameter_types! {
	pub const GetLaminarTokenId: CurrencyId = CurrencyId::LAMI;
}

pub type LaminarToken = BasicCurrencyAdapter<Runtime, pallet_balances::Module<Runtime>, Balance>;

impl orml_currencies::Trait for Runtime {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = LaminarToken;
	type GetNativeCurrencyId = GetLaminarTokenId;
}

impl orml_prices::Trait for Runtime {
	type CurrencyId = CurrencyId;
	type Source = orml_oracle::Module<Runtime>;
}

impl synthetic_tokens::Trait for Runtime {
	type Event = Event;
	type CurrencyId = CurrencyId;
	type Balance = Balance;
	type LiquidityPoolId = LiquidityPoolId;
}

parameter_types! {
	pub const GetLiquidityCurrencyId: CurrencyId = CurrencyId::AUSD;
}

type LiquidityCurrency = orml_currencies::Currency<Runtime, GetLiquidityCurrencyId>;

pub struct PoolManager;

impl LiquidityPoolManager<LiquidityPoolId> for PoolManager {
	fn can_remove(_pool_id: LiquidityPoolId) -> bool {
		true
	}
}

impl liquidity_pools::Trait for Runtime {
	type Event = Event;
	type MultiCurrency = orml_currencies::Module<Runtime>;
	type LiquidityCurrency = LiquidityCurrency;
	type LiquidityPoolId = LiquidityPoolId;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type PoolManager = PoolManager;
	type ExistentialDeposit = ExistentialDeposit;
}

parameter_types! {
	pub const GetCollateralCurrencyId: CurrencyId = CurrencyId::AUSD;
}
type CollateralCurrency = orml_currencies::Currency<Runtime, GetCollateralCurrencyId>;
impl synthetic_protocol::Trait for Runtime {
	type Event = Event;
	type MultiCurrency = orml_currencies::Module<Runtime>;
	type CollateralCurrency = CollateralCurrency;
	type GetCollateralCurrencyId = GetCollateralCurrencyId;
	type PriceProvider = orml_prices::Module<Runtime>;
	type LiquidityPools = liquidity_pools::Module<Runtime>;
	type BalanceToPrice = BalancePriceConverter;
	type PriceToBalance = BalancePriceConverter;
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: system::{Module, Call, Storage, Config, Event},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		Aura: pallet_aura::{Module, Config<T>, Inherent(Timestamp)},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event},
		Indices: pallet_indices,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment::{Module, Storage},
		Sudo: pallet_sudo,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		OperatorCollective: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		OperatorMembership: pallet_membership::<Instance1>::{Module, Call, Storage, Event<T>, Config<T>},
		Oracle: orml_oracle::{Module, Storage, Call, Event<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		Prices: orml_prices::{Module, Storage},
		SyntheticTokens: synthetic_tokens::{Module, Storage, Call, Event<T>},
		SyntheticProtocol: synthetic_protocol::{Module, Call, Event<T>},
		LiquidityPools: liquidity_pools::{Module, Storage, Call, Event<T>},
	}
);

/// The address format for describing accounts.
pub type Address = <Indices as StaticLookup>::Source;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	system::CheckVersion<Runtime>,
	system::CheckGenesis<Runtime>,
	system::CheckEra<Runtime>,
	system::CheckNonce<Runtime>,
	system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<Runtime, Block, system::ChainContext<Runtime>, Runtime, AllModules>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(tx: <Block as BlockT>::Extrinsic) -> TransactionValidity {
			Executive::validate_transaction(tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> u64 {
			Aura::slot_duration()
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities()
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}
	}

	impl system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
		UncheckedExtrinsic,
	> for Runtime {
		fn query_info(uxt: UncheckedExtrinsic, len: u32) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
	}
}
