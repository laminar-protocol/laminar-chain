//! The LaminarChain dev runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod benchmarking;
mod constants;
pub mod tests;

use codec::Encode;
use pallet_collective::{EnsureMembers, EnsureProportionMoreThan};
use pallet_grandpa::fg_primitives;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_session::historical as pallet_session_historical;
use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_core::{
	crypto::KeyTypeId,
	u32_trait::{_1, _2, _3, _4},
};
use sp_runtime::traits::{
	BlakeTwo256, Block as BlockT, Convert, NumberFor, OpaqueKeys, SaturatedConversion, StaticLookup,
};
use sp_runtime::{
	create_runtime_str,
	curve::PiecewiseLinear,
	generic, impl_opaque_keys,
	traits::{Extrinsic, Saturating, Verify},
	transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, FixedPointNumber, ModuleId,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use frame_system::{Call as SystemCall, EnsureOneOf, EnsureRoot};
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::DataProvider;
pub use primitives::{
	AccountId, AccountIndex, Amount, Balance, BlockNumber, CurrencyId, EraIndex, Hash, LiquidityPoolId, Moment, Nonce,
	Price, Signature,
};
pub use sp_arithmetic::FixedI128;

use margin_protocol_rpc_runtime_api::{MarginPoolState, MarginTraderState};
use pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo;
use synthetic_protocol_rpc_runtime_api::SyntheticPoolState;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, debug, parameter_types,
	traits::{Contains, ContainsLengthBound, Filter, InstanceFilter, KeyOwnerProofSystem, Randomness},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		Weight,
	},
	StorageValue,
};
pub use pallet_staking::StakerStatus;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Percent, Permill};

pub use constants::{currency::*, fee::*, time::*};

use cumulus_primitives::{relay_chain::Balance as RelayChainBalance, ParaId};

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
			pub grandpa: Grandpa,
			pub babe: Babe,
		}
	}
}

pub use constants::time::*;

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("laminar"),
	impl_name: create_runtime_str!("laminar"),
	authoring_version: 1,
	spec_version: 202,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// The version infromation used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

pub struct BaseFilter;
impl Filter<Call> for BaseFilter {
	fn filter(_call: &Call) -> bool {
		true
	}
}

const AVERAGE_ON_INITIALIZE_WEIGHT: Perbill = Perbill::from_percent(10);
parameter_types! {
	pub const BlockHashCount: BlockNumber = 900; // mortal tx can be valid up to 1 hour after signing
	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub const MaximumBlockWeight: Weight = 2 * WEIGHT_PER_SECOND;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	/// Assume 10% of weight for average on_initialize calls.
	pub MaximumExtrinsicWeight: Weight =
		AvailableBlockRatio::get().saturating_sub(AVERAGE_ON_INITIALIZE_WEIGHT)
		* MaximumBlockWeight::get();
	pub const MaximumBlockLength: u32 = 5 * 1024 * 1024;
	pub const Version: RuntimeVersion = VERSION;
}

impl frame_system::Trait for Runtime {
	type BaseCallFilter = BaseFilter;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = Indices;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Nonce;
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
	/// Maximum weight of each extrinsic.
	type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
	/// Maximum weight of each block.
	type MaximumBlockWeight = MaximumBlockWeight;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// The weight of the overhead invoked on the block import process, independent of the
	/// extrinsics included in that block.
	type BlockExecutionWeight = BlockExecutionWeight;
	/// The base weight of any extrinsic processed by the runtime, independent of the
	/// logic of that extrinsic. (Signature verification, nonce increment, fee, etc...)
	type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
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
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
}

impl pallet_utility::Trait for Runtime {
	type Event = Event;
	type Call = Call;
	type WeightInfo = ();
}

parameter_types! {
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_SLOTS;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Trait for Runtime {
	type Event = Event;
	type Call = Call;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = ();
}

impl pallet_babe::Trait for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;

	type KeyOwnerProofSystem = Historical;

	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;

	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::IdentificationTuple;

	type HandleEquivocation = pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences>;
}

impl pallet_grandpa::Trait for Runtime {
	type Event = Event;
	type Call = Call;

	type KeyOwnerProofSystem = Historical;

	type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::IdentificationTuple;

	type HandleEquivocation = pallet_grandpa::EquivocationHandler<Self::KeyOwnerIdentification, Offences>;
}

parameter_types! {
	/// How much an index costs.
	pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pallet_indices::Trait for Runtime {
	/// The type for recording indexing into the account enumeration. If this ever overflows, there
	/// will be problems!
	type AccountIndex = AccountIndex;
	/// The ubiquitous event type.
	type Event = Event;
	/// The currency type.
	type Currency = Balances;
	/// How much an index costs.
	type Deposit = IndexDeposit;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Trait for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = (Staking, ()); // ImOnline
}

parameter_types! {
	pub const LamiExistentialDeposit: Balance = 100 * MILLICENTS;
}

impl pallet_balances::Trait for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type Event = Event;
	type ExistentialDeposit = LamiExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
}

impl pallet_transaction_payment::Trait for Runtime {
	type Currency = pallet_balances::Module<Runtime>;
	type OnTransactionPayment = ();
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = WeightToFee;
	type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Trait for Runtime {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_SLOTS as _;
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	pub const OracleUnsignedPriority: TransactionPriority = TransactionPriority::max_value() - 1;
	pub const MarginProtocolUnsignedPriority: TransactionPriority = TransactionPriority::max_value() - 2;
}

parameter_types! {
	pub OffencesWeightSoftLimit: Weight = Perbill::from_percent(60) * MaximumBlockWeight::get();
}

impl pallet_offences::Trait for Runtime {
	type Event = Event;
	type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
	type OnOffenceHandler = Staking;
	type WeightSoftLimit = OffencesWeightSoftLimit;
	type WeightInfo = ();
}

parameter_types! {
	pub const GeneralCouncilMotionDuration: BlockNumber = 1 * DAYS;
	pub const GeneralCouncilMaxProposals: u32 = 100;
}

type GeneralCouncilInstance = pallet_collective::Instance1;
impl pallet_collective::Trait<GeneralCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = GeneralCouncilMotionDuration;
	type MaxProposals = GeneralCouncilMaxProposals;
	type WeightInfo = ();
}

type EnsureThreeFourthGeneralCouncilOrRoot =
	EnsureOneOf<AccountId, EnsureProportionMoreThan<_3, _4, AccountId, GeneralCouncilInstance>, EnsureRoot<AccountId>>;

type EnsureHalfGeneralCouncilOrRoot =
	EnsureOneOf<AccountId, EnsureProportionMoreThan<_1, _2, AccountId, GeneralCouncilInstance>, EnsureRoot<AccountId>>;

type EnsureOneThirdGeneralCouncilOrRoot =
	EnsureOneOf<AccountId, EnsureProportionMoreThan<_1, _3, AccountId, GeneralCouncilInstance>, EnsureRoot<AccountId>>;

type GeneralCouncilMembershipInstance = pallet_membership::Instance1;
impl pallet_membership::Trait<GeneralCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureThreeFourthGeneralCouncilOrRoot;
	type RemoveOrigin = EnsureThreeFourthGeneralCouncilOrRoot;
	type SwapOrigin = EnsureThreeFourthGeneralCouncilOrRoot;
	type ResetOrigin = EnsureThreeFourthGeneralCouncilOrRoot;
	type PrimeOrigin = EnsureHalfGeneralCouncilOrRoot;
	type MembershipInitialized = GeneralCouncil;
	type MembershipChanged = GeneralCouncil;
}

parameter_types! {
	pub const FinancialCouncilMotionDuration: BlockNumber = 1 * DAYS;
	pub const FinancialCouncilMaxProposals: u32 = 100;
}

type FinancialCouncilInstance = pallet_collective::Instance2;
impl pallet_collective::Trait<FinancialCouncilInstance> for Runtime {
	type Origin = Origin;
	type Proposal = Call;
	type Event = Event;
	type MotionDuration = FinancialCouncilMotionDuration;
	type MaxProposals = FinancialCouncilMaxProposals;
	type WeightInfo = ();
}

type EnsureHalfFinancialCouncilOrRoot = EnsureOneOf<
	AccountId,
	EnsureProportionMoreThan<_1, _2, AccountId, FinancialCouncilInstance>,
	EnsureRoot<AccountId>,
>;

type FinancialCouncilMembershipInstance = pallet_membership::Instance2;
impl pallet_membership::Trait<FinancialCouncilMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureHalfFinancialCouncilOrRoot;
	type RemoveOrigin = EnsureHalfFinancialCouncilOrRoot;
	type SwapOrigin = EnsureHalfFinancialCouncilOrRoot;
	type ResetOrigin = EnsureHalfFinancialCouncilOrRoot;
	type PrimeOrigin = EnsureHalfFinancialCouncilOrRoot;
	type MembershipInitialized = FinancialCouncil;
	type MembershipChanged = FinancialCouncil;
}

type OperatorMembershipInstance = pallet_membership::Instance3;
impl pallet_membership::Trait<OperatorMembershipInstance> for Runtime {
	type Event = Event;
	type AddOrigin = EnsureOneThirdGeneralCouncilOrRoot;
	type RemoveOrigin = EnsureOneThirdGeneralCouncilOrRoot;
	type SwapOrigin = EnsureOneThirdGeneralCouncilOrRoot;
	type ResetOrigin = EnsureOneThirdGeneralCouncilOrRoot;
	type PrimeOrigin = EnsureHalfGeneralCouncilOrRoot;
	type MembershipInitialized = Oracle;
	type MembershipChanged = Oracle;
}

pub struct GeneralCouncilProvider;
impl Contains<AccountId> for GeneralCouncilProvider {
	fn contains(who: &AccountId) -> bool {
		GeneralCouncil::is_member(who)
	}

	fn sorted_members() -> Vec<AccountId> {
		GeneralCouncil::members()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add(_who: &AccountId) {}
}

impl ContainsLengthBound for GeneralCouncilProvider {
	fn max_len() -> usize {
		100
	}
	fn min_len() -> usize {
		0
	}
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(10);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const TipReportDepositPerByte: Balance = 1 * CENTS;
	pub const TreasuryModuleId: ModuleId = ModuleId(*b"lami/try");
}

impl pallet_treasury::Trait for Runtime {
	type Currency = Balances;
	type ApproveOrigin =
		EnsureOneOf<AccountId, EnsureMembers<_4, AccountId, GeneralCouncilInstance>, EnsureRoot<AccountId>>;
	type RejectOrigin =
		EnsureOneOf<AccountId, EnsureMembers<_2, AccountId, GeneralCouncilInstance>, EnsureRoot<AccountId>>;
	type Tippers = GeneralCouncilProvider;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type TipReportDepositPerByte = TipReportDepositPerByte;
	type Event = Event;
	type ProposalRejection = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type ModuleId = TreasuryModuleId;
	type BurnDestination = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Trait for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Trait>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;
	type ShouldEndSession = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Keys = opaque::SessionKeys;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type NextSessionRotation = Babe;
	type WeightInfo = ();
}

impl pallet_session::historical::Trait for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

/// Struct that handles the conversion of Balance -> `u64`. This is used for staking's election
/// calculation.
pub struct CurrencyToVoteHandler;

impl CurrencyToVoteHandler {
	fn factor() -> Balance {
		(Balances::total_issuance() / u64::max_value() as Balance).max(1)
	}
}

impl Convert<Balance, u64> for CurrencyToVoteHandler {
	fn convert(x: Balance) -> u64 {
		(x / Self::factor()) as u64
	}
}

impl Convert<u128, Balance> for CurrencyToVoteHandler {
	fn convert(x: u128) -> Balance {
		x * Self::factor()
	}
}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = 3; // 3 hours
	pub const BondingDuration: pallet_staking::EraIndex = 4; // 12 hours
	pub const SlashDeferDuration: pallet_staking::EraIndex = 2; // 6 hours
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const ElectionLookahead: BlockNumber = EPOCH_DURATION_IN_BLOCKS / 4;
	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	pub const MaxIterations: u32 = 5;
	// 0.05%. The higher the value, the more strict solution acceptance becomes.
	pub MinSolutionScoreBump: Perbill = Perbill::from_rational_approximation(5u32, 10_000);
}

impl pallet_staking::Trait for Runtime {
	type Currency = Balances;
	type UnixTime = Timestamp;
	type CurrencyToVote = CurrencyToVoteHandler;
	type RewardRemainder = PalletTreasury;
	type Event = Event;
	type Slash = PalletTreasury; // send the slashed funds to the pallet treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = EnsureThreeFourthGeneralCouncilOrRoot;
	type SessionInterface = Self;
	type RewardCurve = RewardCurve;
	type NextNewSession = Session;
	type ElectionLookahead = ElectionLookahead;
	type Call = Call;
	type MaxIterations = MaxIterations;
	type MinSolutionScoreBump = MinSolutionScoreBump;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type UnsignedPriority = StakingUnsignedPriority;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumCount: u32 = 1;
	pub const ExpiresIn: Moment = 1000 * 60 * 60 * 24 * 3; // 3 days
}

impl orml_oracle::Trait for Runtime {
	type Event = Event;
	type OnNewData = ();
	type CombineData = orml_oracle::DefaultCombineData<Runtime, MinimumCount, ExpiresIn>;
	type Time = Timestamp;
	type OracleKey = CurrencyId;
	type OracleValue = Price;
	type UnsignedPriority = OracleUnsignedPriority;
	type AuthorityId = orml_oracle::AuthorityId;
}

pub type TimeStampedPrice = orml_oracle::TimestampedValueOf<Runtime>;

impl orml_tokens::Trait for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type OnReceived = ();
}

parameter_types! {
	pub const GetLaminarTokenId: CurrencyId = CurrencyId::LAMI;
	pub SyntheticCurrencyIds: Vec<CurrencyId> = vec![
		CurrencyId::FEUR,
		CurrencyId::FJPY,
		CurrencyId::FAUD,
		CurrencyId::FCAD,
		CurrencyId::FCHF,
		CurrencyId::FXAU,
		CurrencyId::FOIL,
		CurrencyId::FBTC,
		CurrencyId::FETH,
	];
	pub const DefaultExtremeRatio: Permill = Permill::from_percent(1);
	pub const DefaultLiquidationRatio: Permill = Permill::from_percent(5);
	pub const DefaultCollateralRatio: Permill = Permill::from_percent(10);
}

pub type LaminarToken = BasicCurrencyAdapter<pallet_balances::Module<Runtime>, Balance, Balance, Amount, BlockNumber>;

impl orml_currencies::Trait for Runtime {
	type Event = Event;
	type MultiCurrency = orml_tokens::Module<Runtime>;
	type NativeCurrency = LaminarToken;
	type GetNativeCurrencyId = GetLaminarTokenId;
}

pub struct LaminarDataProvider;
impl DataProvider<CurrencyId, Price> for LaminarDataProvider {
	fn get(currency: &CurrencyId) -> Option<Price> {
		match currency {
			CurrencyId::AUSD => Some(Price::saturating_from_integer(1)),
			_ => <Oracle as DataProvider<CurrencyId, Price>>::get(currency),
		}
	}
}
impl synthetic_tokens::Trait for Runtime {
	type Event = Event;
	type DefaultExtremeRatio = DefaultExtremeRatio;
	type DefaultLiquidationRatio = DefaultLiquidationRatio;
	type DefaultCollateralRatio = DefaultCollateralRatio;
	type SyntheticCurrencyIds = SyntheticCurrencyIds;
	type UpdateOrigin = EnsureHalfFinancialCouncilOrRoot;
}

parameter_types! {
	pub const GetLiquidityCurrencyId: CurrencyId = CurrencyId::AUSD;
	pub MaxSwap: FixedI128 = FixedI128::saturating_from_integer(2); // TODO: set this
}

type LiquidityCurrency = orml_currencies::Currency<Runtime, GetLiquidityCurrencyId>;

pub type BaseLiquidityPoolsMarginInstance = base_liquidity_pools::Instance1;
parameter_types! {
	pub const MarginLiquidityPoolsModuleId: ModuleId = margin_liquidity_pools::MODULE_ID;
	pub const LiquidityPoolExistentialDeposit: Balance = 10 * DOLLARS;
	pub const IdentityDeposit: Balance = 10_000 * DOLLARS;
}

impl base_liquidity_pools::Trait<BaseLiquidityPoolsMarginInstance> for Runtime {
	type Event = Event;
	type LiquidityCurrency = LiquidityCurrency;
	type PoolManager = MarginProtocol;
	type ExistentialDeposit = LiquidityPoolExistentialDeposit;
	type IdentityDeposit = IdentityDeposit;
	type IdentityDepositCurrency = Balances;
	type ModuleId = MarginLiquidityPoolsModuleId;
	type OnDisableLiquidityPool = MarginLiquidityPools;
	type OnRemoveLiquidityPool = MarginLiquidityPools;
	type UpdateOrigin = EnsureHalfFinancialCouncilOrRoot;
}

pub type BaseLiquidityPoolsSyntheticInstance = base_liquidity_pools::Instance2;
parameter_types! {
	pub const SyntheticLiquidityPoolsModuleId: ModuleId = synthetic_liquidity_pools::MODULE_ID;
}
impl base_liquidity_pools::Trait<BaseLiquidityPoolsSyntheticInstance> for Runtime {
	type Event = Event;
	type LiquidityCurrency = LiquidityCurrency;
	type PoolManager = SyntheticTokens;
	type ExistentialDeposit = LiquidityPoolExistentialDeposit;
	type IdentityDeposit = IdentityDeposit;
	type IdentityDepositCurrency = Balances;
	type ModuleId = SyntheticLiquidityPoolsModuleId;
	type OnDisableLiquidityPool = SyntheticLiquidityPools;
	type OnRemoveLiquidityPool = SyntheticLiquidityPools;
	type UpdateOrigin = EnsureHalfFinancialCouncilOrRoot;
}

impl margin_liquidity_pools::Trait for Runtime {
	type Event = Event;
	type BaseLiquidityPools = BaseLiquidityPoolsForMargin;
	type PoolManager = MarginProtocol;
	type UpdateOrigin = EnsureHalfFinancialCouncilOrRoot;
	type MaxSwapRate = MaxSwap;
	type UnixTime = Timestamp;
	type Moment = Moment;
}

impl synthetic_liquidity_pools::Trait for Runtime {
	type Event = Event;
	type BaseLiquidityPools = BaseLiquidityPoolsForSynthetic;
	type UpdateOrigin = EnsureHalfFinancialCouncilOrRoot;
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
	type PriceProvider = orml_traits::DefaultPriceProvider<CurrencyId, LaminarDataProvider>;
	type LiquidityPools = synthetic_liquidity_pools::Module<Runtime>;
	type SyntheticProtocolLiquidityPools = synthetic_liquidity_pools::Module<Runtime>;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
		// take the biggest period possible.
		let period = BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let extra: SignedExtra = (
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				debug::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature.into(), extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
}

parameter_types! {
	pub const GetTraderMaxOpenPositions: usize = 200;
	pub const GetPoolMaxOpenPositions: usize = 1000;
	pub GetTreasuryAccountId: AccountId = pallet_treasury::Module::<Runtime>::account_id();
}

impl margin_protocol::Trait for Runtime {
	type Event = Event;
	type LiquidityCurrency = LiquidityCurrency;
	type LiquidityPools = margin_liquidity_pools::Module<Runtime>;
	type PriceProvider = orml_traits::DefaultPriceProvider<CurrencyId, LaminarDataProvider>;
	type GetTreasuryAccountId = GetTreasuryAccountId;
	type GetTraderMaxOpenPositions = GetTraderMaxOpenPositions;
	type GetPoolMaxOpenPositions = GetPoolMaxOpenPositions;
	type UpdateOrigin = EnsureHalfFinancialCouncilOrRoot;
	type UnsignedPriority = MarginProtocolUnsignedPriority;
}

impl cumulus_parachain_upgrade::Trait for Runtime {
	type Event = Event;
	type OnValidationFunctionParams = ();
}

impl cumulus_message_broker::Trait for Runtime {
	type Event = Event;
	type DownwardMessageHandlers = XTokens;
	type UpwardMessage = cumulus_upward_message::RococoUpwardMessage;
	type ParachainId = ParachainInfo;
	type XCMPMessage = orml_xtokens::XCMPTokenMessage<AccountId, Balance>;
	type XCMPMessageHandlers = XTokens;
}

impl parachain_info::Trait for Runtime {}

parameter_types! {
	pub const RelayChainCurrencyId: CurrencyId = CurrencyId::DOT;
}

pub struct RelayToNative;
impl Convert<RelayChainBalance, Balance> for RelayToNative {
	fn convert(val: u128) -> Balance {
		// native is 18
		// relay is 12
		val * 1_000_000
	}
}

pub struct NativeToRelay;
impl Convert<Balance, RelayChainBalance> for NativeToRelay {
	fn convert(val: u128) -> Balance {
		// native is 18
		// relay is 12
		val / 1_000_000
	}
}

impl orml_xtokens::Trait for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type Currency = Currencies;
	type XCMPMessageSender = MessageBroker;
	type RelayChainCurrencyId = RelayChainCurrencyId;
	type UpwardMessageSender = MessageBroker;
	type FromRelayChainBalance = RelayToNative;
	type ToRelayChainBalance = NativeToRelay;
	type UpwardMessage = cumulus_upward_message::RococoUpwardMessage;
	type ParaId = ParachainInfo;
}

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		Authorship: pallet_authorship::{Module, Call, Storage, Inherent},
		Babe: pallet_babe::{Module, Call, Storage, Config, Inherent, ValidateUnsigned},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event, ValidateUnsigned},
		Indices: pallet_indices::{Module, Call, Storage, Event<T>, Config<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},
		Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
		Offences: pallet_offences::{Module, Call, Storage, Event},
		Historical: pallet_session_historical::{Module},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		GeneralCouncil: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		GeneralCouncilMembership: pallet_membership::<Instance1>::{Module, Call, Storage, Event<T>, Config<T>},
		FinancialCouncil: pallet_collective::<Instance2>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		FinancialCouncilMembership: pallet_membership::<Instance2>::{Module, Call, Storage, Event<T>, Config<T>},
		Oracle: orml_oracle::{Module, Storage, Call, Config<T>, Event<T>, ValidateUnsigned},
		// OperatorMembership must be placed after Oracle or else will have race condition on initialization
		OperatorMembership: pallet_membership::<Instance3>::{Module, Call, Storage, Event<T>, Config<T>},
		Utility: pallet_utility::{Module, Call, Storage, Event},
		Multisig: pallet_multisig::{Module, Call, Storage, Event<T>},
		PalletTreasury: pallet_treasury::{Module, Call, Storage, Config, Event<T>},
		Staking: pallet_staking::{Module, Call, Config<T>, Storage, Event<T>},
		Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
		Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
		Currencies: orml_currencies::{Module, Call, Event<T>},
		SyntheticTokens: synthetic_tokens::{Module, Storage, Call, Event, Config},
		SyntheticProtocol: synthetic_protocol::{Module, Call, Event<T>},
		MarginProtocol: margin_protocol::{Module, Storage, Call, Event<T>, Config, ValidateUnsigned},
		BaseLiquidityPoolsForMargin: base_liquidity_pools::<Instance1>::{Module, Storage, Call, Event<T>},
		MarginLiquidityPools: margin_liquidity_pools::{Module, Storage, Call, Event<T>, Config<T>},
		BaseLiquidityPoolsForSynthetic: base_liquidity_pools::<Instance2>::{Module, Storage, Call, Event<T>},
		SyntheticLiquidityPools: synthetic_liquidity_pools::{Module, Storage, Call, Event<T>, Config},

		// parachain modules
		ParachainUpgrade: cumulus_parachain_upgrade::{Module, Call, Storage, Inherent, Event},
		MessageBroker: cumulus_message_broker::{Module, Call, Inherent, Event<T>},
		ParachainInfo: parachain_info::{Module, Storage, Config},
		XTokens: orml_xtokens::{Module, Storage, Call, Event<T>},
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
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive =
	frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllModules>;

#[cfg(not(feature = "disable-runtime-api"))]
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
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
			// The choice of `c` parameter (where `1 - c` represents the
			// probability of a slot being empty), is done in accordance to the
			// slot duration and expected target block time, for safely
			// resisting network delays of maximum two seconds.
			// <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
			sp_consensus_babe::BabeGenesisConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: PRIMARY_PROBABILITY,
				genesis_authorities: Babe::authorities(),
				randomness: Babe::randomness(),
				allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::SlotNumber {
			Babe::current_epoch_start()
		}

		fn generate_key_ownership_proof(
			_slot_number: sp_consensus_babe::SlotNumber,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
			NumberFor<Block>,
			>,
			key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Grandpa::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
			)
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((fg_primitives::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(fg_primitives::OpaqueKeyOwnershipProof::new)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
	}

	impl orml_oracle_rpc_runtime_api::OracleApi<
		Block,
		CurrencyId,
		TimeStampedPrice,
	> for Runtime {
		fn get_value(key: CurrencyId) -> Option<TimeStampedPrice> {
			Oracle::get_no_op(&key)
		}

		fn get_all_values() -> Vec<(CurrencyId, Option<TimeStampedPrice>)>{
			Oracle::get_all_values()
		}
	}

	impl margin_protocol_rpc_runtime_api::MarginProtocolApi<Block, AccountId> for Runtime {
		fn trader_state(who: AccountId, pool_id: LiquidityPoolId) -> MarginTraderState {
			let equity = MarginProtocol::equity_of_trader(&who, pool_id).unwrap_or_default();
			let margin_held = MarginProtocol::margin_held(&who, pool_id);
			let margin_level = MarginProtocol::margin_level(&who, pool_id).unwrap_or_default();
			let free_margin = MarginProtocol::free_margin(&who, pool_id).unwrap_or_default();
			let unrealized_pl = MarginProtocol::unrealized_pl_of_trader(&who, pool_id).unwrap_or_default();

			MarginTraderState {
				equity,
				margin_held,
				margin_level,
				free_margin,
				unrealized_pl,
			}
		}

		fn pool_state(pool_id: LiquidityPoolId) -> Option<MarginPoolState> {
			let (enp, ell) = MarginProtocol::enp_and_ell(pool_id)?;
			let required_deposit = MarginProtocol::pool_required_deposit(pool_id)?;

			Some(MarginPoolState { enp, ell, required_deposit })
		}
	}

	impl synthetic_protocol_rpc_runtime_api::SyntheticProtocolApi<Block, AccountId> for Runtime {
		fn pool_state(pool_id: LiquidityPoolId, currency_id: CurrencyId) -> Option<SyntheticPoolState> {
			let collateral_ratio = SyntheticProtocol::collateral_ratio(pool_id, currency_id)?;
			let is_safe = SyntheticProtocol::is_safe_collateral_ratio(currency_id, collateral_ratio);

			Some(SyntheticPoolState { collateral_ratio, is_safe })
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn dispatch_benchmark(
			pallet: Vec<u8>,
			benchmark: Vec<u8>,
			lowest_range_values: Vec<u32>,
			highest_range_values: Vec<u32>,
			steps: Vec<u32>,
			repeat: u32,
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch};
			use orml_benchmarking::add_benchmark;

			let whitelist: Vec<Vec<u8>> = vec![];
			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&pallet, &benchmark, &lowest_range_values, &highest_range_values, &steps, repeat, &whitelist);

			add_benchmark!(params, batches, b"base-liquidity-pools", benchmarking::base_liquidity_pools);
			add_benchmark!(params, batches, b"margin-liquidity-pools", benchmarking::margin_liquidity_pools);
			add_benchmark!(params, batches, b"synthetic-liquidity-pools", benchmarking::synthetic_liquidity_pools);
			add_benchmark!(params, batches, b"margin-protocol", benchmarking::margin_protocol);
			add_benchmark!(params, batches, b"synthetic-protocol", benchmarking::synthetic_protocol);

			if batches.is_empty() { return Err("Benchmark not found for this module.".into()) }
			Ok(batches)
		}
	}
}

cumulus_runtime::register_validate_block!(Block, Executive);
