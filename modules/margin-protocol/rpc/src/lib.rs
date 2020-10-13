use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use laminar_primitives::LiquidityPoolId;
pub use margin_protocol_rpc_runtime_api::{
	MarginPoolState, MarginProtocolApi as MarginProtocolRuntimeApi, MarginTraderState,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait MarginProtocolApi<BlockHash, AccountId> {
	#[rpc(name = "margin_traderState")]
	fn trader_state(
		&self,
		who: AccountId,
		pool_id: LiquidityPoolId,
		at: Option<BlockHash>,
	) -> Result<MarginTraderState>;

	#[rpc(name = "margin_poolState")]
	fn pool_state(&self, pool_id: LiquidityPoolId, at: Option<BlockHash>) -> Result<Option<MarginPoolState>>;
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
	fn trader_state(
		&self,
		who: AccountId,
		pool_id: LiquidityPoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<MarginTraderState> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.trader_state(&at, who, pool_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get trader state.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn pool_state(
		&self,
		pool_id: LiquidityPoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Option<MarginPoolState>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.pool_state(&at, pool_id).map_err(|e| RpcError {
			code: ErrorCode::ServerError(Error::RuntimeError.into()),
			message: "Unable to get pool state.".into(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}
