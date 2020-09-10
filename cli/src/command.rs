// Disable the following lints
#![allow(clippy::borrowed_box)]

use crate::cli::{Cli, RelayChainCli, Subcommand};
use codec::Encode;
use cumulus_primitives::ParaId;
use dev_runtime::Block;
use log::info;
use polkadot_parachain::primitives::AccountIdConversion;
use sc_cli::{
	ChainSpec, CliConfiguration, DefaultConfigurationValues, ImportParams, KeystoreParams, NetworkParams, Result, Role,
	RuntimeVersion, SharedParams, SubstrateCli,
};
use sc_service::config::{BasePath, PrometheusConfig};
use service::IdentifyVariant;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{Block as BlockT, Hash as HashT, Header as HeaderT, Zero};
use std::{io::Write, net::SocketAddr, sync::Arc};

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Laminar Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/laminar-protocol/laminar-chain/issues".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		Ok(match id {
			"dev" => Box::new(service::chain_spec::development_testnet_config(
				self.run.parachain_id.map(Into::into),
			)?),
			"local" => Box::new(service::chain_spec::local_testnet_config(
				self.run.parachain_id.map(Into::into),
			)?),
			"" | "turbulence" => Box::new(service::chain_spec::turbulence_testnet_config()?),
			"turbulence-latest" => Box::new(service::chain_spec::latest_turbulence_testnet_config(
				self.run.parachain_id.map(Into::into),
			)?),
			path => Box::new(service::chain_spec::DevChainSpec::from_json_file(
				std::path::PathBuf::from(path),
			)?),
		})
	}

	fn native_runtime_version(_: &Box<dyn sc_service::ChainSpec>) -> &'static RuntimeVersion {
		&service::dev_runtime::VERSION
	}
}

impl SubstrateCli for RelayChainCli {
	fn impl_name() -> String {
		"Laminar Parachain Collator".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		"Laminar parachain collator\n\nThe command-line arguments provided first will be \
		passed to the parachain node, while the arguments provided after -- will be passed \
		to the relaychain node.\n\n\
		rococo-collator [parachain-args] -- [relaychain-args]"
			.into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"https://github.com/laminar-protocol/laminar-chain/issues".into()
	}

	fn copyright_start_year() -> i32 {
		2019
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		polkadot_cli::Cli::from_iter([RelayChainCli::executable_name().to_string()].iter()).load_spec(id)
	}

	fn native_runtime_version(chain_spec: &Box<dyn ChainSpec>) -> &'static RuntimeVersion {
		polkadot_cli::Cli::native_runtime_version(chain_spec)
	}
}

pub fn generate_genesis_state(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Block> {
	let storage = chain_spec.build_storage()?;

	let child_roots = storage.children_default.iter().map(|(sk, child_content)| {
		let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
			child_content.data.clone().into_iter().collect(),
		);
		(sk.clone(), state_root.encode())
	});
	let state_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(
		storage.top.clone().into_iter().chain(child_roots).collect(),
	);

	let extrinsics_root = <<<Block as BlockT>::Header as HeaderT>::Hashing as HashT>::trie_root(Vec::new());

	Ok(Block::new(
		<<Block as BlockT>::Header as HeaderT>::new(
			Zero::zero(),
			extrinsics_root,
			state_root,
			Default::default(),
			Default::default(),
		),
		Default::default(),
	))
}

fn extract_genesis_wasm(chain_spec: &Box<dyn sc_service::ChainSpec>) -> Result<Vec<u8>> {
	let mut storage = chain_spec.build_storage()?;

	storage
		.top
		.remove(sp_core::storage::well_known_keys::CODE)
		.ok_or_else(|| "Could not find wasm file in genesis state!".into())
}

pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	fn set_default_ss58_version(spec: &Box<dyn service::ChainSpec>) {
		use sp_core::crypto::Ss58AddressFormat;

		let ss58_version = if spec.is_reynolds() {
			Ss58AddressFormat::ReynoldsAccount
		} else if spec.is_laminar() {
			Ss58AddressFormat::LaminarAccount
		} else {
			Ss58AddressFormat::SubstrateAccount
		};

		sp_core::crypto::set_default_ss58_version(ss58_version);
	};

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&*cli.run)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.run_node_until_exit(|config| match config.role {
				Role::Light => service::new_light::<service::dev_runtime::RuntimeApi, service::DevExecutor>(config),
				_ => {
					let extension = service::chain_spec::Extensions::try_get(&config.chain_spec);
					let relay_chain_id = extension.map(|e| e.relay_chain.clone());

					if let (Some(parachain_id), Some(relay_chain_id)) = (cli.run.parachain_id, relay_chain_id) {
						// TODO
						let key = Arc::new(sp_core::Pair::generate().0);

						let polkadot_cli = RelayChainCli::new(
							config.base_path.as_ref().map(|x| x.path().join("polkadot")),
							relay_chain_id,
							[RelayChainCli::executable_name().to_string()]
								.iter()
								.chain(cli.relaychain_args.iter()),
						);

						let id = ParaId::from(parachain_id);

						let parachain_account =
							AccountIdConversion::<polkadot_primitives::v0::AccountId>::into_account(&id);

						let block = generate_genesis_state(&config.chain_spec).map_err(|e| format!("{:?}", e))?;
						let genesis_state = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

						let task_executor = config.task_executor.clone();
						let polkadot_config =
							SubstrateCli::create_configuration(&polkadot_cli, &polkadot_cli, task_executor)
								.map_err(|err| format!("Relay chain argument error: {}", err))?;

						info!("Parachain id: {:?}", id);
						info!("Parachain Account: {}", parachain_account);
						info!("Parachain genesis state: {}", genesis_state);
						info!("Is collating: {}", if cli.run.base.validator { "yes" } else { "no" });

						service::new_collator(config, key, polkadot_config, id, cli.run.base.validator).map(|r| r.0)
					} else {
						service::new_full::<service::dev_runtime::RuntimeApi, service::DevExecutor>(config).map(|r| r.0)
					}
				}
			})
		}
		Some(Subcommand::ExportGenesisState(params)) => {
			sc_cli::init_logger("");

			let block = generate_genesis_state(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;
			let header_hex = format!("0x{:?}", HexDisplay::from(&block.header().encode()));

			if let Some(output) = &params.output {
				std::fs::write(output, header_hex)?;
			} else {
				println!("{}", header_hex);
			}

			Ok(())
		}
		Some(Subcommand::ExportGenesisWasm(params)) => {
			sc_cli::init_logger("");

			let wasm_file = extract_genesis_wasm(&cli.load_spec(&params.chain.clone().unwrap_or_default())?)?;

			if let Some(output) = &params.output {
				std::fs::write(output, wasm_file)?;
			} else {
				std::io::stdout().write_all(&wasm_file)?;
			}

			Ok(())
		}
		Some(Subcommand::Base(subcommand)) => {
			let runner = cli.create_runner(subcommand)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.run_subcommand(subcommand, |config| {
				service::new_chain_ops::<dev_runtime::RuntimeApi, service::DevExecutor>(config)
			})
		}

		Some(Subcommand::Inspect(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.sync_run(|config| {
				cmd.run::<service::dev_runtime::Block, service::dev_runtime::RuntimeApi, service::DevExecutor>(config)
			})
		}

		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			let chain_spec = &runner.config().chain_spec;

			set_default_ss58_version(chain_spec);

			runner.sync_run(|config| cmd.run::<service::dev_runtime::Block, service::DevExecutor>(config))
		}
	}
}

impl DefaultConfigurationValues for RelayChainCli {
	fn p2p_listen_port() -> u16 {
		30334
	}

	fn rpc_ws_listen_port() -> u16 {
		9945
	}

	fn rpc_http_listen_port() -> u16 {
		9934
	}

	fn prometheus_listen_port() -> u16 {
		9616
	}
}

impl CliConfiguration<Self> for RelayChainCli {
	fn shared_params(&self) -> &SharedParams {
		self.base.base.shared_params()
	}

	fn import_params(&self) -> Option<&ImportParams> {
		self.base.base.import_params()
	}

	fn network_params(&self) -> Option<&NetworkParams> {
		self.base.base.network_params()
	}

	fn keystore_params(&self) -> Option<&KeystoreParams> {
		self.base.base.keystore_params()
	}

	fn base_path(&self) -> Result<Option<BasePath>> {
		Ok(self
			.shared_params()
			.base_path()
			.or_else(|| self.base_path.clone().map(Into::into)))
	}

	fn rpc_http(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_http(default_listen_port)
	}

	fn rpc_ipc(&self) -> Result<Option<String>> {
		self.base.base.rpc_ipc()
	}

	fn rpc_ws(&self, default_listen_port: u16) -> Result<Option<SocketAddr>> {
		self.base.base.rpc_ws(default_listen_port)
	}

	fn prometheus_config(&self, default_listen_port: u16) -> Result<Option<PrometheusConfig>> {
		self.base.base.prometheus_config(default_listen_port)
	}

	fn init<C: SubstrateCli>(&self) -> Result<()> {
		unreachable!("PolkadotCli is never initialized; qed");
	}

	fn chain_id(&self, is_dev: bool) -> Result<String> {
		let chain_id = self.base.base.chain_id(is_dev)?;

		Ok(if chain_id.is_empty() {
			self.chain_id.clone().unwrap_or_default()
		} else {
			chain_id
		})
	}

	fn role(&self, is_dev: bool) -> Result<sc_service::Role> {
		self.base.base.role(is_dev)
	}

	fn transaction_pool(&self) -> Result<sc_service::config::TransactionPoolOptions> {
		self.base.base.transaction_pool()
	}

	fn state_cache_child_ratio(&self) -> Result<Option<usize>> {
		self.base.base.state_cache_child_ratio()
	}

	fn rpc_methods(&self) -> Result<sc_service::config::RpcMethods> {
		self.base.base.rpc_methods()
	}

	fn rpc_ws_max_connections(&self) -> Result<Option<usize>> {
		self.base.base.rpc_ws_max_connections()
	}

	fn rpc_cors(&self, is_dev: bool) -> Result<Option<Vec<String>>> {
		self.base.base.rpc_cors(is_dev)
	}

	fn telemetry_external_transport(&self) -> Result<Option<sc_service::config::ExtTransport>> {
		self.base.base.telemetry_external_transport()
	}

	fn default_heap_pages(&self) -> Result<Option<u64>> {
		self.base.base.default_heap_pages()
	}

	fn force_authoring(&self) -> Result<bool> {
		self.base.base.force_authoring()
	}

	fn disable_grandpa(&self) -> Result<bool> {
		self.base.base.disable_grandpa()
	}

	fn max_runtime_instances(&self) -> Result<Option<usize>> {
		self.base.base.max_runtime_instances()
	}

	fn announce_block(&self) -> Result<bool> {
		self.base.base.announce_block()
	}
}
