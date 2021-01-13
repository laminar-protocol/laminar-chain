use laminar_primitives::{AccountId, Balance, Block, CurrencyId, DataProviderId, Nonce, Header, Hash, BlockNumber};
use runtime_common::TimeStampedPrice;
use sc_client_api::{Backend as BackendT, BlockchainEvents, KeyIterator};
use sp_api::{CallApiAt, NumberFor, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_consensus::BlockStatus;
use sp_runtime::{
	generic::{BlockId, SignedBlock},
	traits::{BlakeTwo256, Block as BlockT},
	Justification,
};
use sp_storage::{ChildInfo, PrefixedStorageKey, StorageData, StorageKey};
use std::sync::Arc;

/// A set of APIs that polkadot-like runtimes must implement.
pub trait RuntimeApiCollection:
	sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
	+ sp_api::ApiExt<Block, Error = sp_blockchain::Error>
	+ sp_consensus_babe::BabeApi<Block>
	+ sp_finality_grandpa::GrandpaApi<Block>
	+ sp_block_builder::BlockBuilder<Block>
	+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
	+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
	+ orml_oracle_rpc::OracleRuntimeApi<Block, DataProviderId, CurrencyId, TimeStampedPrice>
	+ margin_protocol_rpc::MarginProtocolRuntimeApi<Block, AccountId>
	+ synthetic_protocol_rpc::SyntheticProtocolRuntimeApi<Block, AccountId>
	+ sp_api::Metadata<Block>
	+ sp_offchain::OffchainWorkerApi<Block>
	+ sp_session::SessionKeys<Block>
where
	<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

impl<Api> RuntimeApiCollection for Api
where
	Api: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::ApiExt<Block, Error = sp_blockchain::Error>
		+ sp_consensus_babe::BabeApi<Block>
		+ sp_finality_grandpa::GrandpaApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
		+ orml_oracle_rpc::OracleRuntimeApi<Block, DataProviderId, CurrencyId, TimeStampedPrice>
		+ margin_protocol_rpc::MarginProtocolRuntimeApi<Block, AccountId>
		+ synthetic_protocol_rpc::SyntheticProtocolRuntimeApi<Block, AccountId>
		+ sp_api::Metadata<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_session::SessionKeys<Block>,
	<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

/// Trait that abstracts over all available client implementations.
///
/// For a concrete type there exists [`Client`].
pub trait AbstractClient<Block, Backend>:
	BlockchainEvents<Block>
	+ Sized
	+ Send
	+ Sync
	+ ProvideRuntimeApi<Block>
	+ HeaderBackend<Block>
	+ CallApiAt<Block, Error = sp_blockchain::Error, StateBackend = Backend::State>
where
	Block: BlockT,
	Backend: BackendT<Block>,
	Backend::State: sp_api::StateBackend<BlakeTwo256>,
	Self::Api: RuntimeApiCollection<StateBackend = Backend::State>,
{
}

impl<Block, Backend, Client> AbstractClient<Block, Backend> for Client
where
	Block: BlockT,
	Backend: BackendT<Block>,
	Backend::State: sp_api::StateBackend<BlakeTwo256>,
	Client: BlockchainEvents<Block>
		+ ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ Sized
		+ Send
		+ Sync
		+ CallApiAt<Block, Error = sp_blockchain::Error, StateBackend = Backend::State>,
	Client::Api: RuntimeApiCollection<StateBackend = Backend::State>,
{
}

pub trait ExecuteWithClient {
	/// The return type when calling this instance.
	type Output;

	/// Execute whatever should be executed with the given client instance.
	fn execute_with_client<Client, Api, Backend>(self, client: Arc<Client>) -> Self::Output
	where
		<Api as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
		Backend: sc_client_api::Backend<Block>,
		Backend::State: sp_api::StateBackend<BlakeTwo256>,
		Api: crate::RuntimeApiCollection<StateBackend = Backend::State>,
		Client: AbstractClient<Block, Backend, Api = Api> + 'static;
}

pub trait ClientHandle {
	/// Execute the given something with the client.
	fn execute_with<T: ExecuteWithClient>(&self, t: T) -> T::Output;
}

#[derive(Clone)]
pub enum Client {
	Dev(Arc<crate::FullClient<dev_runtime::RuntimeApi, crate::DevExecutor>>),
}

impl ClientHandle for Client {
	fn execute_with<T: ExecuteWithClient>(&self, t: T) -> T::Output {
		match self {
			Self::Dev(client) => T::execute_with_client::<_, _, crate::FullBackend>(t, client.clone()),
		}
	}
}

impl sc_client_api::UsageProvider<Block> for Client {
	fn usage_info(&self) -> sc_client_api::ClientInfo<Block> {
		match self {
			Self::Dev(client) => client.usage_info(),
		}
	}
}

impl sc_client_api::BlockBackend<Block> for Client {
	fn block_body(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<Vec<<Block as BlockT>::Extrinsic>>> {
		match self {
			Self::Dev(client) => client.block_body(id),
		}
	}

	fn block(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<SignedBlock<Block>>> {
		match self {
			Self::Dev(client) => client.block(id),
		}
	}

	fn block_status(&self, id: &BlockId<Block>) -> sp_blockchain::Result<BlockStatus> {
		match self {
			Self::Dev(client) => client.block_status(id),
		}
	}

	fn justification(&self, id: &BlockId<Block>) -> sp_blockchain::Result<Option<Justification>> {
		match self {
			Self::Dev(client) => client.justification(id),
		}
	}

	fn block_hash(&self, number: NumberFor<Block>) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			Self::Dev(client) => client.block_hash(number),
		}
	}
}

impl sc_client_api::StorageProvider<Block, crate::FullBackend> for Client {
	fn storage(&self, id: &BlockId<Block>, key: &StorageKey) -> sp_blockchain::Result<Option<StorageData>> {
		match self {
			Self::Dev(client) => client.storage(id, key),
		}
	}

	fn storage_keys(&self, id: &BlockId<Block>, key_prefix: &StorageKey) -> sp_blockchain::Result<Vec<StorageKey>> {
		match self {
			Self::Dev(client) => client.storage_keys(id, key_prefix),
		}
	}

	fn storage_hash(
		&self,
		id: &BlockId<Block>,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			Self::Dev(client) => client.storage_hash(id, key),
		}
	}

	fn storage_pairs(
		&self,
		id: &BlockId<Block>,
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<(StorageKey, StorageData)>> {
		match self {
			Self::Dev(client) => client.storage_pairs(id, key_prefix),
		}
	}

	fn storage_keys_iter<'a>(
		&self,
		id: &BlockId<Block>,
		prefix: Option<&'a StorageKey>,
		start_key: Option<&StorageKey>,
	) -> sp_blockchain::Result<KeyIterator<'a, <crate::FullBackend as sc_client_api::Backend<Block>>::State, Block>> {
		match self {
			Self::Dev(client) => client.storage_keys_iter(id, prefix, start_key),
		}
	}

	fn child_storage(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<StorageData>> {
		match self {
			Self::Dev(client) => client.child_storage(id, child_info, key),
		}
	}

	fn child_storage_keys(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key_prefix: &StorageKey,
	) -> sp_blockchain::Result<Vec<StorageKey>> {
		match self {
			Self::Dev(client) => client.child_storage_keys(id, child_info, key_prefix),
		}
	}

	fn child_storage_hash(
		&self,
		id: &BlockId<Block>,
		child_info: &ChildInfo,
		key: &StorageKey,
	) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
		match self {
			Self::Dev(client) => client.child_storage_hash(id, child_info, key),
		}
	}

	fn max_key_changes_range(
		&self,
		first: NumberFor<Block>,
		last: BlockId<Block>,
	) -> sp_blockchain::Result<Option<(NumberFor<Block>, BlockId<Block>)>> {
		match self {
			Self::Dev(client) => client.max_key_changes_range(first, last),
		}
	}

	fn key_changes(
		&self,
		first: NumberFor<Block>,
		last: BlockId<Block>,
		storage_key: Option<&PrefixedStorageKey>,
		key: &StorageKey,
	) -> sp_blockchain::Result<Vec<(NumberFor<Block>, u32)>> {
		match self {
			Self::Dev(client) => client.key_changes(first, last, storage_key, key),
		}
	}
}

impl sp_blockchain::HeaderBackend<Block> for Client {
	fn header(&self, id: BlockId<Block>) -> sp_blockchain::Result<Option<Header>> {
		match self {
			Self::Dev(client) => client.header(&id),
		}
	}

	fn info(&self) -> sp_blockchain::Info<Block> {
		match self {
			Self::Dev(client) => client.info(),
		}
	}

	fn status(&self, id: BlockId<Block>) -> sp_blockchain::Result<sp_blockchain::BlockStatus> {
		match self {
			Self::Dev(client) => client.status(id),
		}
	}

	fn number(&self, hash: Hash) -> sp_blockchain::Result<Option<BlockNumber>> {
		match self {
			Self::Dev(client) => client.number(hash),
		}
	}

	fn hash(&self, number: BlockNumber) -> sp_blockchain::Result<Option<Hash>> {
		match self {
			Self::Dev(client) => client.hash(number),
		}
	}
}
