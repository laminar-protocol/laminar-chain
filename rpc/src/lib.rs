use std::sync::Arc;

use primitives::{AccountId, Balance, Block, CurrencyId, DataProviderId, Hash, Nonce};
use sc_client_api::light::{Fetcher, RemoteBlockchain};
pub use sc_rpc::DenyUnsafe;
pub use sc_rpc::SubscriptionTaskExecutor;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_transaction_pool::TransactionPool;

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

/// Light client extra dependencies.
pub struct LightDeps<C, F, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Remote access to the blockchain (async).
	pub remote_blockchain: Arc<dyn RemoteBlockchain<Block>>,
	/// Fetcher instance.
	pub fetcher: Arc<F>,
}

/// Full client dependencies.
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P>(deps: FullDeps<C, P>) -> RpcExtension
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: orml_oracle_rpc::OracleRuntimeApi<Block, DataProviderId, CurrencyId, dev_runtime::TimeStampedPrice>,
	C::Api: margin_protocol_rpc::MarginProtocolRuntimeApi<Block, AccountId>,
	C::Api: synthetic_protocol_rpc::SyntheticProtocolRuntimeApi<Block, AccountId>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + Sync + Send + 'static,
{
	use margin_protocol_rpc::{MarginProtocol, MarginProtocolApi};
	use orml_oracle_rpc::{Oracle, OracleApi};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
	use substrate_frame_rpc_system::{FullSystem, SystemApi};
	use synthetic_protocol_rpc::{SyntheticProtocol, SyntheticProtocolApi};

	let mut io = jsonrpc_core::IoHandler::default();
	let FullDeps {
		client,
		pool,
		deny_unsafe,
	} = deps;

	io.extend_with(SystemApi::to_delegate(FullSystem::new(
		client.clone(),
		pool,
		deny_unsafe,
	)));
	io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
		client.clone(),
	)));
	io.extend_with(OracleApi::to_delegate(Oracle::new(client.clone())));
	io.extend_with(MarginProtocolApi::to_delegate(MarginProtocol::new(client.clone())));
	io.extend_with(SyntheticProtocolApi::to_delegate(SyntheticProtocol::new(client)));

	io
}

/// Instantiate all RPC extensions for light node.
pub fn create_light<C, P, F>(deps: LightDeps<C, F, P>) -> RpcExtension
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	F: Fetcher<Block> + 'static,
	P: TransactionPool + 'static,
{
	use substrate_frame_rpc_system::{LightSystem, SystemApi};

	let LightDeps {
		client,
		pool,
		remote_blockchain,
		fetcher,
	} = deps;
	let mut io = jsonrpc_core::IoHandler::default();
	io.extend_with(SystemApi::<Hash, AccountId, Nonce>::to_delegate(LightSystem::new(
		client,
		remote_blockchain,
		fetcher,
		pool,
	)));

	io
}
