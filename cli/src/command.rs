// Disable the following lints
#![allow(clippy::borrowed_box)]

use crate::cli::{Cli, RelayChainCli, Subcommand};
use codec::Encode;
use dev_runtime::Block;
use sc_cli::{Result, Role, RuntimeVersion, SubstrateCli};
use sc_service::ChainSpec;
use service::IdentifyVariant;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{Block as BlockT, Hash as HashT, Header as HeaderT, Zero};
use std::io::Write;

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
				self.run.parachain_id.unwrap_or(5001).into(),
			)?),
			"local" => Box::new(service::chain_spec::local_testnet_config(
				self.run.parachain_id.unwrap_or(5001).into(),
			)?),
			"" | "turbulence" => Box::new(service::chain_spec::turbulence_testnet_config()?),
			"turbulence-latest" => Box::new(service::chain_spec::latest_turbulence_testnet_config(
				self.run.parachain_id.unwrap_or(5001).into(),
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
				_ => service::new_full::<service::dev_runtime::RuntimeApi, service::DevExecutor>(config).map(|r| r.0),
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
