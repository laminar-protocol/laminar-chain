use dev_runtime::{
	opaque::SessionKeys, AccountId, BabeConfig, BalancesConfig, BandOracleConfig, Block, CurrencyId,
	FinancialCouncilMembershipConfig, GeneralCouncilMembershipConfig, GenesisConfig, GrandpaConfig, IndicesConfig,
	LaminarOracleConfig, MarginLiquidityPoolsConfig, MarginProtocolConfig, Moment, OperatorMembershipBandConfig,
	OperatorMembershipLaminarConfig, Price, SessionConfig, Signature, StakerStatus, StakingConfig, SudoConfig,
	SyntheticLiquidityPoolsConfig, SyntheticTokensConfig, SystemConfig, TokensConfig, DOLLARS, WASM_BINARY,
};
use hex_literal::hex;
use laminar_primitives::{AccumulateConfig, SwapRate, TradingPair};
use margin_protocol::RiskThreshold;
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use serde_json::map::Map;
use sp_arithmetic::FixedI128;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_runtime::{FixedPointNumber, Perbill, Permill};
use synthetic_tokens::SyntheticTokensRatio;

type AccountPublic = <Signature as Verify>::Signer;

// The URL for the telemetry server.
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `DevChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type DevChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

fn session_keys(grandpa: GrandpaId, babe: BabeId) -> SessionKeys {
	SessionKeys { grandpa, babe }
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate an Aura authority key.
pub fn authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, GrandpaId, BabeId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
	)
}

pub fn development_testnet_config() -> Result<DevChainSpec, String> {
	let mut properties = Map::new();
	properties.insert("tokenSymbol".into(), "LAMI".into());
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(DevChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Development,
		move || {
			dev_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
				],
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		Some(properties),
		// Extensions
		Default::default(),
	))
}

pub fn local_testnet_config() -> Result<DevChainSpec, String> {
	let mut properties = Map::new();
	properties.insert("tokenSymbol".into(), "LAMI".into());
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(DevChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		move || {
			dev_genesis(
				wasm_binary,
				// Initial PoA authorities
				vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
				// Sudo account
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				// Pre-funded accounts
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
			)
		},
		// Bootnodes
		vec![],
		// Telemetry
		None,
		// Protocol ID
		None,
		// Properties
		Some(properties),
		// Extensions
		Default::default(),
	))
}

pub fn turbulence_testnet_config() -> Result<DevChainSpec, String> {
	DevChainSpec::from_json_bytes(&include_bytes!("../../resources/turbulence-dist.json")[..])
}

pub fn latest_turbulence_testnet_config() -> Result<DevChainSpec, String> {
	let mut properties = Map::new();
	properties.insert("tokenSymbol".into(), "LAMI".into());
	properties.insert("tokenDecimals".into(), 18.into());

	let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm binary not available".to_string())?;

	Ok(DevChainSpec::from_genesis(
		"Laminar Turbulence TC3",
		"turbulence3",
		ChainType::Live,
		// SECRET="..."
		// ./target/debug/subkey inspect "$SECRET//laminar//root"
		// ./target/debug/subkey --sr25519 inspect "$SECRET//laminar//oracle"
		// ./target/debug/subkey --sr25519 inspect "$SECRET//laminar//1//validator"
		// ./target/debug/subkey --sr25519 inspect "$SECRET//laminar//1//babe"
		// ./target/debug/subkey --ed25519 inspect "$SECRET//laminar//1//grandpa"
		// ./target/debug/subkey --sr25519 inspect "$SECRET//laminar//2//validator"
		// ./target/debug/subkey --sr25519 inspect "$SECRET//laminar//2//babe"
		// ./target/debug/subkey --ed25519 inspect "$SECRET//laminar//2//grandpa"
		// ./target/debug/subkey --sr25519 inspect "$SECRET//laminar//3//validator"
		// ./target/debug/subkey --sr25519 inspect "$SECRET//laminar//3//babe"
		// ./target/debug/subkey --ed25519 inspect "$SECRET//laminar//3//grandpa"
		move || turbulence_genesis(
			wasm_binary,
			// Initial PoA authorities
			vec![
				(
					// 5E6jm6dgDZQBFW79gd3uvTKymjqUSzAPfkvD7Exx5GvdbHZ6
					hex!["5a055df2cbdebc8fce61a70db71fcf64c1853dca54d8c3e52b2d65cb8cf7e533"].into(),
					hex!["5a055df2cbdebc8fce61a70db71fcf64c1853dca54d8c3e52b2d65cb8cf7e533"].into(),
					hex!["b48963cb1572aa90e4202db400e7b5aa887b3c6aaf7e61de3a6beb14dae2c97b"].unchecked_into(),
					hex!["f2415a6cedee17c766c7e8f696fb3499519d85a3248b05de35bc7b58d59e4149"].unchecked_into(),
				),
				(
					// 5GGqathCVPRvwTTMEvURf2f16iKu4i8SccxCc6UNGDF4g447
					hex!["ba31e4b5576a5d60b2dbdb4d4144f6478636b84313fe6f41a44e002ddc64ec6c"].into(),
					hex!["ba31e4b5576a5d60b2dbdb4d4144f6478636b84313fe6f41a44e002ddc64ec6c"].into(),
					hex!["293bd01494343a94520531d844953e947e4a1ff84bdae948565e49bdf3304c09"].unchecked_into(),
					hex!["cade610afbc4ce7ca0c6972f5c774c2c4710eed431cc23ac6e5e806870a8dd02"].unchecked_into(),
				),
				(
					// 5GmrbvqhDBp7jmaRB5SsiY5kfkLPXMbELm6MTVsMpbCX19tD
					hex!["d0536fc56cac85d6b61e128becdc367e8d7652d9a95663c7e88cb6119aea966d"].into(),
					hex!["d0536fc56cac85d6b61e128becdc367e8d7652d9a95663c7e88cb6119aea966d"].into(),
					hex!["849c1ea65bc37705aafd4e753fde8395612e9da8d88240d27b2dfc4a2e115599"].unchecked_into(),
					hex!["d84cdabe21cead3f88de87b63116405182cf78ef97d3d590011bc235a983447a"].unchecked_into(),
				),
			],
			// Sudo account
			// 5FySxAHYXDzgDY8BTVnbZ6dygkXJwG27pKmgCLeSRSFEG2dy
			hex!["acee87f3026e9ef8cf334fe94bc9eb9e9e689318611eca21e5aef919e3e5bc30"].into(),
			// Pre-funded accounts
			vec![
				// 5FySxAHYXDzgDY8BTVnbZ6dygkXJwG27pKmgCLeSRSFEG2dy
				hex!["acee87f3026e9ef8cf334fe94bc9eb9e9e689318611eca21e5aef919e3e5bc30"].into(),
				// 5DyXntuH5dBcf2dpjTojzfV6GDypx8CyTuVFm84qB7a4BkYT
				hex!["54865b9eff8c291658e3fbda202f4260536618c31a0056372d121a5206010d53"].into(),
			],
		),
		// Bootnodes
		vec![
			"/dns4/testnet-bootnode-1.laminar-chain.laminar.one/tcp/30333/p2p/12D3KooWNCe9dEpPhswckrX5ZHhdtZ3r5sg6CcgKfgyhw3seuwtB".parse().unwrap(),
		],
		// Telemetry
		Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Staging telemetry url is valid; qed")),
		// Protocol ID
		Some("turbulence2"),
		// Properties
		Some(properties),
		// Extensions
		Default::default(),
	))
}

const INITIAL_BALANCE: u128 = 1_000_000 * DOLLARS;
const INITIAL_STAKING: u128 = 100_000 * DOLLARS;
const HOURS_IN_SECONDS: u64 = 60 * 60;

const EUR_USD: TradingPair = TradingPair {
	base: CurrencyId::FEUR,
	quote: CurrencyId::AUSD,
};
const USD_JPY: TradingPair = TradingPair {
	base: CurrencyId::AUSD,
	quote: CurrencyId::FJPY,
};
const AUD_USD: TradingPair = TradingPair {
	base: CurrencyId::FAUD,
	quote: CurrencyId::AUSD,
};
const USD_CAD: TradingPair = TradingPair {
	base: CurrencyId::AUSD,
	quote: CurrencyId::FCAD,
};
const USD_CHF: TradingPair = TradingPair {
	base: CurrencyId::AUSD,
	quote: CurrencyId::FCHF,
};
const XAU_USD: TradingPair = TradingPair {
	base: CurrencyId::FXAU,
	quote: CurrencyId::AUSD,
};
const USD_OIL: TradingPair = TradingPair {
	base: CurrencyId::AUSD,
	quote: CurrencyId::FOIL,
};
const BTC_USD: TradingPair = TradingPair {
	base: CurrencyId::FBTC,
	quote: CurrencyId::AUSD,
};
const ETH_USD: TradingPair = TradingPair {
	base: CurrencyId::FETH,
	quote: CurrencyId::AUSD,
};

fn accumulate_config(frequency: Moment, offset: Moment) -> AccumulateConfig<Moment> {
	AccumulateConfig { frequency, offset }
}

fn risk_threshold(margin_call_percent: u32, stop_out_percent: u32) -> RiskThreshold {
	RiskThreshold {
		margin_call: Permill::from_percent(margin_call_percent),
		stop_out: Permill::from_percent(stop_out_percent),
	}
}

fn dev_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
	GenesisConfig {
		frame_system: Some(SystemConfig {
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_indices: Some(IndicesConfig { indices: vec![] }),
		pallet_balances: Some(BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, INITIAL_BALANCE)).collect(),
		}),
		pallet_session: Some(SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), session_keys(x.2.clone(), x.3.clone())))
				.collect::<Vec<_>>(),
		}),
		pallet_staking: Some(StakingConfig {
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), INITIAL_STAKING, StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		}),
		pallet_sudo: Some(SudoConfig { key: root_key.clone() }),
		pallet_babe: Some(BabeConfig { authorities: vec![] }),
		pallet_grandpa: Some(GrandpaConfig { authorities: vec![] }),
		pallet_collective_Instance1: Some(Default::default()),
		pallet_membership_Instance1: Some(GeneralCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		}),
		pallet_collective_Instance2: Some(Default::default()),
		pallet_membership_Instance2: Some(FinancialCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		}),
		pallet_treasury: Some(Default::default()),
		orml_tokens: Some(TokensConfig {
			endowed_accounts: endowed_accounts
				.iter()
				.flat_map(|x| vec![(x.clone(), CurrencyId::AUSD, INITIAL_BALANCE)])
				.collect(),
		}),
		synthetic_liquidity_pools: Some(SyntheticLiquidityPoolsConfig {
			min_additional_collateral_ratio: Permill::from_percent(10), // default min additional collateral ratio
		}),
		synthetic_tokens: Some(SyntheticTokensConfig {
			ratios: vec![
				(
					CurrencyId::FOIL,
					SyntheticTokensRatio {
						extreme: Some(Permill::from_percent(5)),
						liquidation: Some(Permill::from_percent(10)),
						collateral: Some(Permill::from_percent(50)),
					},
				),
				(
					CurrencyId::FBTC,
					SyntheticTokensRatio {
						extreme: Some(Permill::from_percent(5)),
						liquidation: Some(Permill::from_percent(10)),
						collateral: Some(Permill::from_percent(50)),
					},
				),
				(
					CurrencyId::FETH,
					SyntheticTokensRatio {
						extreme: Some(Permill::from_percent(5)),
						liquidation: Some(Permill::from_percent(10)),
						collateral: Some(Permill::from_percent(50)),
					},
				),
			],
		}),
		margin_liquidity_pools: Some(MarginLiquidityPoolsConfig {
			default_min_leveraged_amount: DOLLARS,
			margin_liquidity_config: vec![
				(
					// TradingPair
					EUR_USD,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					USD_JPY,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					AUD_USD,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					USD_CAD,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					USD_CHF,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					XAU_USD,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					USD_OIL,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					BTC_USD,
					// MaxSpread
					Price::from_inner(20),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 1000),
						short: FixedI128::saturating_from_rational(1, 1000),
					},
				),
				(
					// TradingPair
					ETH_USD,
					// MaxSpread
					Price::from_inner(1),
					// Accumulates
					accumulate_config(HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 1000),
						short: FixedI128::saturating_from_rational(1, 1000),
					},
				),
			],
		}),
		margin_protocol: Some(MarginProtocolConfig {
			risk_thresholds: vec![
				(
					EUR_USD,
					// TraderRiskThreshold
					risk_threshold(7, 4),
					// LiquidityPoolENPThreshold
					risk_threshold(60, 30),
					// LiquidityPoolELLThreshold
					risk_threshold(20, 5),
				),
				(
					USD_JPY,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					AUD_USD,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					USD_CAD,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					USD_CHF,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					XAU_USD,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					USD_OIL,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					BTC_USD,
					risk_threshold(15, 7),
					risk_threshold(80, 40),
					risk_threshold(40, 12),
				),
				(
					ETH_USD,
					risk_threshold(15, 7),
					risk_threshold(80, 40),
					risk_threshold(40, 12),
				),
			],
		}),
		orml_oracle_Instance1: Some(LaminarOracleConfig {
			members: Default::default(), // initialized by OperatorMembership
			phantom: Default::default(),
		}),
		orml_oracle_Instance2: Some(BandOracleConfig {
			members: Default::default(), // initialized by OperatorMembership
			phantom: Default::default(),
		}),
		pallet_membership_Instance3: Some(OperatorMembershipLaminarConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		}),
		pallet_membership_Instance4: Some(OperatorMembershipBandConfig {
			members: vec![root_key],
			phantom: Default::default(),
		}),
	}
}

fn turbulence_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
	GenesisConfig {
		frame_system: Some(SystemConfig {
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_indices: Some(IndicesConfig { indices: vec![] }),
		pallet_balances: Some(BalancesConfig {
			balances: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), INITIAL_STAKING + DOLLARS)) // add bit more for tx fee
				.chain(endowed_accounts.iter().cloned().map(|k| (k, INITIAL_BALANCE)))
				.collect(),
		}),
		pallet_session: Some(SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), session_keys(x.2.clone(), x.3.clone())))
				.collect::<Vec<_>>(),
		}),
		pallet_staking: Some(StakingConfig {
			validator_count: 5,
			minimum_validator_count: 1,
			stakers: initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), INITIAL_STAKING, StakerStatus::Validator))
				.collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		}),
		pallet_sudo: Some(SudoConfig { key: root_key.clone() }),
		pallet_babe: Some(BabeConfig { authorities: vec![] }),
		pallet_grandpa: Some(GrandpaConfig { authorities: vec![] }),
		pallet_collective_Instance1: Some(Default::default()),
		pallet_membership_Instance1: Some(GeneralCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		}),
		pallet_collective_Instance2: Some(Default::default()),
		pallet_membership_Instance2: Some(FinancialCouncilMembershipConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		}),
		pallet_treasury: Some(Default::default()),
		orml_tokens: Some(TokensConfig {
			endowed_accounts: endowed_accounts
				.iter()
				.flat_map(|x| vec![(x.clone(), CurrencyId::AUSD, INITIAL_BALANCE)])
				.collect(),
		}),
		synthetic_liquidity_pools: Some(SyntheticLiquidityPoolsConfig {
			min_additional_collateral_ratio: Permill::from_percent(10), // default min additional collateral ratio
		}),
		synthetic_tokens: Some(SyntheticTokensConfig {
			ratios: vec![
				(
					CurrencyId::FOIL,
					SyntheticTokensRatio {
						extreme: Some(Permill::from_percent(5)),
						liquidation: Some(Permill::from_percent(10)),
						collateral: Some(Permill::from_percent(50)),
					},
				),
				(
					CurrencyId::FBTC,
					SyntheticTokensRatio {
						extreme: Some(Permill::from_percent(5)),
						liquidation: Some(Permill::from_percent(10)),
						collateral: Some(Permill::from_percent(50)),
					},
				),
				(
					CurrencyId::FETH,
					SyntheticTokensRatio {
						extreme: Some(Permill::from_percent(5)),
						liquidation: Some(Permill::from_percent(10)),
						collateral: Some(Permill::from_percent(50)),
					},
				),
			],
		}),
		margin_liquidity_pools: Some(MarginLiquidityPoolsConfig {
			default_min_leveraged_amount: DOLLARS,
			margin_liquidity_config: vec![
				(
					// TradingPair
					EUR_USD,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(24 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					USD_JPY,
					// MaxSpread
					Price::from_fraction(0.001),
					// Accumulates
					accumulate_config(24 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					AUD_USD,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(24 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					USD_CAD,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(24 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					USD_CHF,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(24 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					XAU_USD,
					// MaxSpread
					Price::from_inner(1),
					// Accumulates
					accumulate_config(24 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					USD_OIL,
					// MaxSpread
					Price::from_fraction(0.01),
					// Accumulates
					accumulate_config(24 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 10000),
						short: FixedI128::saturating_from_rational(-1, 10000),
					},
				),
				(
					// TradingPair
					BTC_USD,
					// MaxSpread
					Price::from_inner(20),
					// Accumulates
					accumulate_config(8 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 1000),
						short: FixedI128::saturating_from_rational(1, 1000),
					},
				),
				(
					// TradingPair
					ETH_USD,
					// MaxSpread
					Price::from_inner(1),
					// Accumulates
					accumulate_config(8 * HOURS_IN_SECONDS, 0),
					// SwapRates
					SwapRate {
						long: FixedI128::saturating_from_rational(1, 1000),
						short: FixedI128::saturating_from_rational(1, 1000),
					},
				),
			],
		}),
		margin_protocol: Some(MarginProtocolConfig {
			risk_thresholds: vec![
				(
					EUR_USD,
					// TraderRiskThreshold
					risk_threshold(7, 4),
					// LiquidityPoolENPThreshold
					risk_threshold(60, 30),
					// LiquidityPoolELLThreshold
					risk_threshold(20, 5),
				),
				(
					USD_JPY,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					AUD_USD,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					USD_CAD,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					USD_CHF,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					XAU_USD,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					USD_OIL,
					risk_threshold(7, 4),
					risk_threshold(60, 30),
					risk_threshold(20, 5),
				),
				(
					BTC_USD,
					risk_threshold(15, 7),
					risk_threshold(80, 40),
					risk_threshold(40, 12),
				),
				(
					ETH_USD,
					risk_threshold(15, 7),
					risk_threshold(80, 40),
					risk_threshold(40, 12),
				),
			],
		}),
		orml_oracle_Instance1: Some(LaminarOracleConfig {
			members: Default::default(), // initialized by OperatorMembership
			phantom: Default::default(),
		}),
		orml_oracle_Instance2: Some(BandOracleConfig {
			members: Default::default(), // initialized by OperatorMembership
			phantom: Default::default(),
		}),
		pallet_membership_Instance3: Some(OperatorMembershipLaminarConfig {
			members: vec![root_key.clone()],
			phantom: Default::default(),
		}),
		pallet_membership_Instance4: Some(OperatorMembershipBandConfig {
			members: vec![root_key],
			phantom: Default::default(),
		}),
	}
}
