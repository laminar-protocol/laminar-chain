use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use margin_protocol_rpc_runtime_api::{MarginProtocolApi as MarginProtocolRuntimeApi, PoolInfo, TraderInfo};
use module_primitives::LiquidityPoolId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait MarginProtocolApi<BlockHash, AccountId> {
	#[rpc(name = "margin_traderInfo")]
	fn trader_info(&self, who: AccountId, at: Option<BlockHash>) -> Result<TraderInfo>;

	#[rpc(name = "margin_poolInfo")]
	fn pool_info(&self, pool_id: LiquidityPoolId, at: Option<BlockHash>) -> Result<Option<PoolInfo>>;
}

/// A struct that implements the [`MarginProtocolApi`].
pub struct MarginProtocol<C, B> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<B>,
}

impl<C, B> MarginProtocol<C, B> {
	/// Create new `MarginProtocol` with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: Default::default(),
		}
	}
}

pub enum Error {
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
		}
	}
}

impl<C, Block, AccountId> MarginProtocolApi<<Block as BlockT>::Hash, AccountId> for MarginProtocol<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: MarginProtocolRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn trader_info(&self, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<TraderInfo> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.trader_info(&at, who)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get trader info.".into(),
				data: Some(format!("{:?}", e).into()),
			})
			.into()
	}

	fn pool_info(&self, pool_id: LiquidityPoolId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<PoolInfo>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.pool_info(&at, pool_id)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get pool info.".into(),
				data: Some(format!("{:?}", e).into()),
			})
			.into()
	}
}
