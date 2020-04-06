use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use margin_protocol_rpc_runtime_api::MarginProtocolApi as MarginProtocolRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

#[rpc]
pub trait MarginProtocolApi<BlockHash, AccountId, Fixed128> {
	#[rpc(name = "marginProtocol_equity_of_trader")]
	fn equity_of_trader(&self, who: AccountId, at: Option<BlockHash>) -> Result<Option<Fixed128>>;

	#[rpc(name = "marginProtocol_margin_level")]
	fn margin_level(&self, who: AccountId, at: Option<BlockHash>) -> Result<Option<Fixed128>>;

	#[rpc(name = "marginProtocol_free_margin")]
	fn free_margin(&self, who: AccountId, at: Option<BlockHash>) -> Result<Option<Fixed128>>;

	#[rpc(name = "marginProtocol_margin_held")]
	fn margin_held(&self, who: AccountId, at: Option<BlockHash>) -> Result<Fixed128>;

	#[rpc(name = "marginProtocol_unrealized_pl_of_trader")]
	fn unrealized_pl_of_trader(&self, who: AccountId, at: Option<BlockHash>) -> Result<Option<Fixed128>>;
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

impl<C, Block, AccountId, Fixed128> MarginProtocolApi<<Block as BlockT>::Hash, AccountId, Fixed128>
	for MarginProtocol<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: MarginProtocolRuntimeApi<Block, AccountId, Fixed128>,
	AccountId: Codec,
	Fixed128: Codec,
{
	fn equity_of_trader(&self, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<Fixed128>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.equity_of_trader(&at, who)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get equity.".into(),
				data: Some(format!("{:?}", e).into()),
			})
			.into()
	}

	fn margin_level(&self, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<Fixed128>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.margin_level(&at, who)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get margin level.".into(),
				data: Some(format!("{:?}", e).into()),
			})
			.into()
	}

	fn free_margin(&self, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<Fixed128>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.free_margin(&at, who)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get free margin.".into(),
				data: Some(format!("{:?}", e).into()),
			})
			.into()
	}

	fn margin_held(&self, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Fixed128> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.margin_held(&at, who)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get margin held.".into(),
				data: Some(format!("{:?}", e).into()),
			})
			.into()
	}

	fn unrealized_pl_of_trader(&self, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> Result<Option<Fixed128>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
			// If the block hash is not supplied assume the best block.
			self.client.info().best_hash));
		api.unrealized_pl_of_trader(&at, who)
			.map_err(|e| RpcError {
				code: ErrorCode::ServerError(Error::RuntimeError.into()),
				message: "Unable to get unrealized P/L of opening positions.".into(),
				data: Some(format!("{:?}", e).into()),
			})
			.into()
	}
}
