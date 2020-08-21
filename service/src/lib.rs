use ansi_term::Color;
use std::sync::Arc;

use cumulus_network::DelayedBlockAnnounceValidator;
use cumulus_service::{prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams};
use laminar_primitives::{AccountId, Balance, Block, CurrencyId, Nonce};
use polkadot_primitives::v0::CollatorPair;
use sc_finality_grandpa::FinalityProofProvider as GrandpaFinalityProofProvider;
use sc_informant::OutputFormat;
use sc_service::{
	config::{Configuration, PrometheusConfig},
	error::Error as ServiceError,
	TaskManager,
};
use sp_runtime::traits::BlakeTwo256;

pub use dev_runtime;
pub use sc_executor::NativeExecutionDispatch;
pub use sc_service::ChainSpec;
pub use sp_api::ConstructRuntimeApi;

pub mod chain_spec;
pub mod executor;

pub use crate::executor::DevExecutor;

/// Can be called for a `Configuration` to check if it is a configuration for
/// the `Laminar` network.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Laminar` network.
	fn is_laminar(&self) -> bool;

	/// Returns if this is a configuration for the `Reynolds` network.
	fn is_reynolds(&self) -> bool;

	/// Returns if this is a configuration for the `Turbulence` network.
	fn is_turbulence(&self) -> bool;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_laminar(&self) -> bool {
		self.id().starts_with("lami")
	}

	fn is_reynolds(&self) -> bool {
		self.id().starts_with("rey")
	}

	fn is_turbulence(&self) -> bool {
		self.id().starts_with("tur")
	}
}

pub trait RuntimeApiCollection:
	sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
	+ sp_api::ApiExt<Block, Error = sp_blockchain::Error>
	+ sp_consensus_babe::BabeApi<Block>
	+ sp_finality_grandpa::GrandpaApi<Block>
	+ sp_block_builder::BlockBuilder<Block>
	+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
	+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
	+ orml_oracle_rpc::OracleRuntimeApi<Block, CurrencyId, dev_runtime::TimeStampedPrice>
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
		+ orml_oracle_rpc::OracleRuntimeApi<Block, CurrencyId, dev_runtime::TimeStampedPrice>
		+ margin_protocol_rpc::MarginProtocolRuntimeApi<Block, AccountId>
		+ synthetic_protocol_rpc::SyntheticProtocolRuntimeApi<Block, AccountId>
		+ sp_api::Metadata<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_session::SessionKeys<Block>,
	<Self as sp_api::ApiExt<Block>>::StateBackend: sp_api::StateBackend<BlakeTwo256>,
{
}

fn set_prometheus_registry(config: &mut Configuration) -> Result<(), ServiceError> {
	if let Some(PrometheusConfig { registry, .. }) = config.prometheus_config.as_mut() {
		*registry = prometheus_endpoint::Registry::new_custom(Some("laminar".into()), None)?;
	}

	Ok(())
}

type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullClient<RuntimeApi, Executor> = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullGrandpaBlockImport<RuntimeApi, Executor> =
	sc_finality_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient<RuntimeApi, Executor>, FullSelectChain>;

type LightBackend = sc_service::TLightBackendWithHash<Block, sp_runtime::traits::BlakeTwo256>;

type LightClient<RuntimeApi, Executor> = sc_service::TLightClientWithBackend<Block, RuntimeApi, Executor, LightBackend>;

fn new_partial<RuntimeApi, Executor>(
	config: &mut Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		FullSelectChain,
		sp_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(
			impl Fn(laminar_rpc::DenyUnsafe) -> laminar_rpc::RpcExtension,
			(
				sc_consensus_babe::BabeBlockImport<
					Block,
					FullClient<RuntimeApi, Executor>,
					FullGrandpaBlockImport<RuntimeApi, Executor>,
				>,
				sc_finality_grandpa::LinkHalf<Block, FullClient<RuntimeApi, Executor>, FullSelectChain>,
				sc_consensus_babe::BabeLink<Block>,
			),
			sc_finality_grandpa::SharedVoterState,
		),
	>,
	sc_service::Error,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	set_prometheus_registry(config)?;

	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let (client, backend, keystore, task_manager) = sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import_with_authority_set_hard_forks(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		Vec::new(),
	)?;

	let justification_import = grandpa_block_import.clone();

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get_or_compute(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;

	let import_queue = sc_consensus_babe::import_queue(
		babe_link.clone(),
		block_import.clone(),
		Some(Box::new(justification_import)),
		None,
		client.clone(),
		select_chain.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
	)?;

	let shared_authority_set = grandpa_link.shared_authority_set().clone();
	let shared_voter_state = sc_finality_grandpa::SharedVoterState::empty();

	let import_setup = (block_import.clone(), grandpa_link, babe_link.clone());
	let rpc_setup = shared_voter_state.clone();

	let babe_config = babe_link.config().clone();
	let shared_epoch_changes = babe_link.epoch_changes().clone();

	let rpc_extensions_builder = {
		let client = client.clone();
		let keystore = keystore.clone();
		let transaction_pool = transaction_pool.clone();
		let select_chain = select_chain.clone();

		move |deny_unsafe| -> laminar_rpc::RpcExtension {
			let deps = laminar_rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				select_chain: select_chain.clone(),
				deny_unsafe,
				babe: Some(laminar_rpc::BabeDeps {
					babe_config: babe_config.clone(),
					shared_epoch_changes: shared_epoch_changes.clone(),
					keystore: keystore.clone(),
				}),
				grandpa: Some(laminar_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
				}),
			};

			laminar_rpc::create_full(deps)
		}
	};

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore,
		select_chain,
		import_queue,
		transaction_pool,
		inherent_data_providers,
		other: (rpc_extensions_builder, import_setup, rpc_setup),
	})
}

pub fn new_full<RuntimeApi, Executor>(
	mut config: Configuration,
) -> Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>), sc_service::Error>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	use sc_client_api::ExecutorProvider;
	use sp_core::traits::BareCryptoStorePtr;

	let role = config.role.clone();
	let is_authority = role.is_authority();
	let force_authoring = config.force_authoring;
	let disable_grandpa = config.disable_grandpa;
	let name = config.network.node_name.clone();

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		keystore,
		select_chain,
		import_queue,
		transaction_pool,
		inherent_data_providers,
		other: (rpc_extensions_builder, import_setup, rpc_setup),
	} = new_partial::<RuntimeApi, Executor>(&mut config)?;

	let prometheus_registry = config.prometheus_registry().cloned();

	let finality_proof_provider = GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
			finality_proof_request_builder: None,
			finality_proof_provider: Some(finality_proof_provider.clone()),
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			backend.clone(),
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let telemetry_connection_sinks = sc_service::TelemetryConnectionSinks::default();

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config: config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore.clone(),
		network: network.clone(),
		rpc_extensions_builder: Box::new(rpc_extensions_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		on_demand: None,
		remote_blockchain: None,
		telemetry_connection_sinks: telemetry_connection_sinks.clone(),
		network_status_sinks,
		system_rpc_tx,
	})?;

	let (block_import, link_half, babe_link) = import_setup;

	let shared_voter_state = rpc_setup;

	if role.is_authority() {
		let can_author_with = sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let proposer =
			sc_basic_authorship::ProposerFactory::new(client.clone(), transaction_pool, prometheus_registry.as_ref());

		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore.clone(),
			client: client.clone(),
			select_chain,
			block_import,
			env: proposer,
			sync_oracle: network.clone(),
			inherent_data_providers: inherent_data_providers.clone(),
			force_authoring,
			babe_link,
			can_author_with,
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;
		task_manager.spawn_essential_handle().spawn_blocking("babe", babe);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore = if is_authority {
		Some(keystore.clone() as BareCryptoStorePtr)
	} else {
		None
	};

	let config = sc_finality_grandpa::Config {
		// FIXME substrate#1578 make this available through chainspec
		gossip_duration: std::time::Duration::from_millis(1000),
		justification_period: 512,
		name: Some(name),
		observer_enabled: false,
		keystore,
		is_authority: role.is_network_authority(),
	};

	let enable_grandpa = !disable_grandpa;
	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: unlike in substrate we are currently running the full
		// GRANDPA voter protocol for all full nodes (regardless of whether
		// they're validators or not). at this point the full voter should
		// provide better guarantees of block and vote data availability than
		// the observer.

		let grandpa_config = sc_finality_grandpa::GrandpaParams {
			config,
			link: link_half,
			network: network.clone(),
			inherent_data_providers: inherent_data_providers.clone(),
			telemetry_on_connect: Some(telemetry_connection_sinks.on_connect_stream()),
			voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state,
		};

		task_manager
			.spawn_essential_handle()
			.spawn_blocking("grandpa-voter", sc_finality_grandpa::run_grandpa_voter(grandpa_config)?);
	} else {
		sc_finality_grandpa::setup_disabled_grandpa(client.clone(), &inherent_data_providers, network.clone())?;
	}

	network_starter.start_network();

	Ok((task_manager, client))
}

pub struct FullNodeHandles;

/// Builds a new service for a light client.
pub fn new_light<Runtime, Dispatch>(mut config: Configuration) -> Result<TaskManager, sc_service::Error>
where
	Runtime: 'static + Send + Sync + ConstructRuntimeApi<Block, LightClient<Runtime, Dispatch>>,
	<Runtime as ConstructRuntimeApi<Block, LightClient<Runtime, Dispatch>>>::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<LightBackend, Block>>,
	Dispatch: NativeExecutionDispatch + 'static,
{
	crate::set_prometheus_registry(&mut config)?;
	use sc_client_api::backend::RemoteBackend;

	let (client, backend, keystore, mut task_manager, on_demand) =
		sc_service::new_light_parts::<Block, Runtime, Dispatch>(&config)?;

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
		on_demand.clone(),
	));

	let grandpa_block_import = sc_finality_grandpa::light_block_import(
		client.clone(),
		backend.clone(),
		&(client.clone() as Arc<_>),
		Arc::new(on_demand.checker().clone()),
	)?;

	let finality_proof_import = grandpa_block_import.clone();
	let finality_proof_request_builder = finality_proof_import.create_finality_proof_request_builder();

	let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get_or_compute(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;

	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	// FIXME: pruning task isn't started since light client doesn't do `AuthoritySetup`.
	let import_queue = sc_consensus_babe::import_queue(
		babe_link,
		babe_block_import,
		None,
		Some(Box::new(finality_proof_import)),
		client.clone(),
		select_chain.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
	)?;

	let finality_proof_provider = GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: Some(on_demand.clone()),
			block_announce_validator_builder: None,
			finality_proof_request_builder: Some(finality_proof_request_builder),
			finality_proof_provider: Some(finality_proof_provider),
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			backend.clone(),
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let light_deps = laminar_rpc::LightDeps {
		remote_blockchain: backend.remote_blockchain(),
		fetcher: on_demand.clone(),
		client: client.clone(),
		pool: transaction_pool.clone(),
	};

	let rpc_extensions = laminar_rpc::create_light(light_deps);

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		on_demand: Some(on_demand),
		remote_blockchain: Some(backend.remote_blockchain()),
		rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
		task_manager: &mut task_manager,
		telemetry_connection_sinks: sc_service::TelemetryConnectionSinks::default(),
		config,
		keystore,
		backend,
		transaction_pool,
		client,
		network,
		network_status_sinks,
		system_rpc_tx,
	})?;

	network_starter.start_network();

	Ok(task_manager)
}

pub fn new_chain_ops<Runtime, Dispatch>(
	mut config: Configuration,
) -> Result<
	(
		Arc<FullClient<Runtime, Dispatch>>,
		Arc<FullBackend>,
		sp_consensus::import_queue::BasicQueue<Block, sp_trie::PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
	),
	ServiceError,
>
where
	Runtime: ConstructRuntimeApi<Block, FullClient<Runtime, Dispatch>> + Send + Sync + 'static,
	Runtime::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	Dispatch: NativeExecutionDispatch + 'static,
{
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	let sc_service::PartialComponents {
		client,
		backend,
		import_queue,
		task_manager,
		..
	} = new_partial::<Runtime, Dispatch>(&mut config)?;
	Ok((client, backend, import_queue, task_manager))
}

fn new_collator_partial<RuntimeApi, Executor>(
	config: &mut Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		(),
		sp_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		impl Fn(laminar_rpc::DenyUnsafe) -> laminar_rpc::RpcExtension,
	>,
	sc_service::Error,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	set_prometheus_registry(config)?;

	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let (client, backend, keystore, task_manager) = sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
	let client = Arc::new(client);

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	let registry = config.prometheus_registry();

	let import_queue = cumulus_consensus::import_queue::import_queue(
		client.clone(),
		client.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		registry.clone(),
	)?;

	let rpc_extensions_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();
		let select_chain = sc_consensus::LongestChain::new(backend.clone());

		move |deny_unsafe| -> laminar_rpc::RpcExtension {
			let deps = laminar_rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				select_chain: select_chain.clone(),
				deny_unsafe,
				babe: None,
				grandpa: None,
			};

			laminar_rpc::create_full(deps)
		}
	};

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore,
		select_chain: (),
		import_queue,
		transaction_pool,
		inherent_data_providers,
		other: rpc_extensions_builder,
	})
}

fn new_collator_impl<RuntimeApi, Executor>(
	parachain_config: Configuration,
	collator_key: Arc<CollatorPair>,
	mut polkadot_config: polkadot_collator::Configuration,
	id: polkadot_primitives::v0::Id,
	validator: bool,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	sc_client_api::StateBackendFor<FullBackend, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
{
	let mut parachain_config = prepare_node_config(parachain_config);

	parachain_config.informant_output_format = OutputFormat {
		enable_color: true,
		prefix: format!("[{}] ", Color::Yellow.bold().paint("Parachain")),
	};
	polkadot_config.informant_output_format = OutputFormat {
		enable_color: true,
		prefix: format!("[{}] ", Color::Blue.bold().paint("Relaychain")),
	};

	let params = new_collator_partial::<RuntimeApi, Executor>(&mut parachain_config)?;
	params
		.inherent_data_providers
		.register_provider(sp_timestamp::InherentDataProvider)
		.unwrap();

	let client = params.client.clone();
	let backend = params.backend.clone();
	let block_announce_validator = DelayedBlockAnnounceValidator::new();
	let block_announce_validator_builder = {
		let block_announce_validator = block_announce_validator.clone();
		move |_| Box::new(block_announce_validator) as Box<_>
	};

	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let mut task_manager = params.task_manager;
	let import_queue = params.import_queue;
	let (network, network_status_sinks, system_rpc_tx, start_network) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &parachain_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: Some(Box::new(block_announce_validator_builder)),
			finality_proof_request_builder: None,
			finality_proof_provider: None,
		})?;

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		on_demand: None,
		remote_blockchain: None,
		rpc_extensions_builder: Box::new(params.other),
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		telemetry_connection_sinks: Default::default(),
		config: parachain_config,
		keystore: params.keystore,
		backend,
		network: network.clone(),
		network_status_sinks,
		system_rpc_tx,
	})?;

	let announce_block = Arc::new(move |hash, data| network.announce_block(hash, data));

	if validator {
		let proposer_factory =
			sc_basic_authorship::ProposerFactory::new(client.clone(), transaction_pool, prometheus_registry.as_ref());

		let params = StartCollatorParams {
			para_id: id,
			block_import: client.clone(),
			proposer_factory,
			inherent_data_providers: params.inherent_data_providers,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			block_announce_validator,
			task_manager: &mut task_manager,
			polkadot_config,
			collator_key,
		};

		start_collator(params)?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			polkadot_config,
			collator_key,
			block_announce_validator,
			task_manager: &mut task_manager,
			para_id: id,
		};

		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

pub fn new_collator(
	parachain_config: Configuration,
	collator_key: Arc<CollatorPair>,
	polkadot_config: polkadot_collator::Configuration,
	id: polkadot_primitives::v0::Id,
	validator: bool,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<dev_runtime::RuntimeApi, DevExecutor>>)> {
	new_collator_impl::<dev_runtime::RuntimeApi, DevExecutor>(
		parachain_config,
		collator_key,
		polkadot_config,
		id,
		validator,
	)
}
