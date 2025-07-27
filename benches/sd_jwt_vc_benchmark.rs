use criterion::{
    black_box, criterion_group, criterion_main, Criterion, BenchmarkId, measurement::WallTime,
    BatchSize,
};

use std::{
    time::{Duration, Instant},
    collections::HashMap,
};
use serde_json::Value;

const MEASUREMENT_TIME_SECS: u64 = 10;
const SAMPLE_SIZE: usize = 20;

use sd_vc_qr::{
    wallet::{
        Wallet, dilithium2::Dilithium2Wallet, secp256k1::Secp256k1Wallet,
        p256::P256Wallet, ed25519::Ed25519Wallet, falcon512::Falcon512Wallet,
        sphincsplus128s::SphincsPLus128sWallet,
        IssuanceParams, IssuanceConfig, FieldsAmount, FieldsSize,
    },
};

#[derive(Clone, Copy, Debug)]
struct BenchConfig {
    wallet_type: WalletType,
    fields_amount: FieldsAmount,
    fields_size: FieldsSize,
}

#[derive(Clone, Copy, Debug)]
enum WalletType {
    Dilithium2,
    Secp256k1,
    P256,
    Ed25519,
    Falcon512,
    SphincsPLus128s,
}

impl std::fmt::Display for WalletType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalletType::Dilithium2 => write!(f, "Dilithium2"),
            WalletType::Secp256k1 => write!(f, "Secp256k1"),
            WalletType::P256 => write!(f, "P256"),
            WalletType::Ed25519 => write!(f, "Ed25519"),
            WalletType::Falcon512 => write!(f, "Falcon512"),
            WalletType::SphincsPLus128s => write!(f, "SPHINCS+128s"),
        }
    }
}

impl std::fmt::Display for BenchConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}_{}",
            self.wallet_type,
            self.fields_amount,
            self.fields_size
        )
    }
}

#[derive(Debug, Default)]
struct BenchMetrics {
    did_gen_time_ms: f64,
    issuance_time_ms: f64,
    verification_time_ms: f64,
    credential_size_bytes: usize,
    field_count: usize,
    disclosures_size_bytes: usize,
}

#[derive(Debug, serde::Serialize)]
struct SizeMetrics {
    algorithm: String,
    field_size: String,
    credential_size_bytes: usize,
    field_count: usize,
    disclosures_size_bytes: usize,
}

fn get_wallet_for_config(config: &BenchConfig) -> Box<dyn Wallet> {
    match config.wallet_type {
        WalletType::Dilithium2 => Box::new(Dilithium2Wallet {}),
        WalletType::Secp256k1 => Box::new(Secp256k1Wallet {}),
        WalletType::P256 => Box::new(P256Wallet {}),
        WalletType::Ed25519 => Box::new(Ed25519Wallet {}),
        WalletType::Falcon512 => Box::new(Falcon512Wallet {}),
        WalletType::SphincsPLus128s => Box::new(SphincsPLus128sWallet {}),
    }
}

fn run_benchmark(config: &BenchConfig) -> BenchMetrics {
    let mut metrics = BenchMetrics::default();

    let wallet = get_wallet_for_config(config);
    
    let did_start = Instant::now();
    let (issuer_did, keys) = wallet.generate_did().unwrap();
    metrics.did_gen_time_ms = did_start.elapsed().as_secs_f64() * 1000.0;
    
    let issuance_params = IssuanceParams {
        issuer_did,
        method: "BenchmarkCredential".to_string(),
        private_key: keys.priv_key,
        config: IssuanceConfig {
            fields_amount: config.fields_amount,
            fields_size: config.fields_size,
            demo_vc: false,
            seed: 42,
        },
    };

    let issuance_start = Instant::now();
    let (credential, disclosures) = wallet.issue_sd_vc_jwt(issuance_params).unwrap();
    metrics.issuance_time_ms = issuance_start.elapsed().as_secs_f64() * 1000.0;
    
    metrics.credential_size_bytes = credential.len();
    
    if let Value::Array(disc_arr) = disclosures {
        metrics.field_count = disc_arr.len().saturating_sub(1);
        metrics.disclosures_size_bytes = serde_json::to_string(&disc_arr).unwrap().len();
    }

    let verification_start = Instant::now();
    let is_valid = wallet.verify_sd_vc_jwt(credential).unwrap();
    metrics.verification_time_ms = verification_start.elapsed().as_secs_f64() * 1000.0;
    
    assert!(is_valid, "Credential verification failed");
    
    metrics
}

fn generate_configs() -> Vec<BenchConfig> {
    let wallet_types = [WalletType::Dilithium2, WalletType::Secp256k1, WalletType::P256, WalletType::Ed25519, WalletType::Falcon512, WalletType::SphincsPLus128s];
    let field_amounts = [FieldsAmount::Small, FieldsAmount::Medium, FieldsAmount::Large];
    let field_sizes = [FieldsSize::Small, FieldsSize::Medium, FieldsSize::Large];
    
    let mut configs = Vec::new();
    
    for &wallet_type in &wallet_types {
        for &fields_amount in &field_amounts {
            for &fields_size in &field_sizes {
                configs.push(BenchConfig {
                    wallet_type,
                    fields_amount,
                    fields_size,
                });
            }
        }
    }
    
    configs
}

fn bench_issue_sd_jwt_vc(c: &mut Criterion<WallTime>) {
    let configs = generate_configs();
    
    let mut group = c.benchmark_group("sd_jwt_vc_issuance");
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sample_size(SAMPLE_SIZE);
    
    for config in &configs {
        group.bench_function(
            BenchmarkId::new("issue", config.to_string()), 
            |b| {
                b.iter_batched(
                    || config.clone(),
                    |cfg| {
                        let wallet = get_wallet_for_config(&cfg);
                        let (issuer_did, keys) = wallet.generate_did().unwrap();
                        let params = IssuanceParams {
                            issuer_did,
                            method: "BenchmarkCredential".to_string(),
                            private_key: keys.priv_key,
                            config: IssuanceConfig {
                                fields_amount: cfg.fields_amount,
                                fields_size: cfg.fields_size,
                                demo_vc: false,
                                seed: 42,
                            },
                        };
                        black_box(wallet.issue_sd_vc_jwt(params).unwrap())
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    
    group.finish();
}

fn bench_verify_sd_jwt_vc(c: &mut Criterion<WallTime>) {
    let configs = generate_configs();
    
    let mut group = c.benchmark_group("sd_jwt_vc_verification");
    group.measurement_time(Duration::from_secs(MEASUREMENT_TIME_SECS));
    group.sample_size(SAMPLE_SIZE);
    
    for config in &configs {
        group.bench_function(
            BenchmarkId::new("verify", config.to_string()), 
            |b| {
                let wallet = get_wallet_for_config(&config);
                let (issuer_did, keys) = wallet.generate_did().unwrap();
                let params = IssuanceParams {
                    issuer_did,
                    method: "BenchmarkCredential".to_string(),
                    private_key: keys.priv_key.clone(),
                    config: IssuanceConfig {
                        fields_amount: config.fields_amount,
                        fields_size: config.fields_size,
                        demo_vc: false,
                        seed: 42,
                    },
                };
                let (credential, _) = wallet.issue_sd_vc_jwt(params).unwrap();

                
                b.iter(|| {
                    black_box(wallet.verify_sd_vc_jwt(credential.clone()).unwrap())
                })
            },
        );
    }
    
    group.finish();
}

fn collect_size_data() -> HashMap<String, SizeMetrics> {
    let configs = generate_configs();
    let mut results = HashMap::new();
    
    println!("\n=== Collecting Size Data ===");
    
    for config in &configs {
        println!("Collecting size data for: {}", config);

        
        let wallet = get_wallet_for_config(config);
        let (issuer_did, keys) = wallet.generate_did().unwrap();
        let params = IssuanceParams {
            issuer_did,
            method: "BenchmarkCredential".to_string(),
            private_key: keys.priv_key,
            config: IssuanceConfig {
                fields_amount: config.fields_amount,
                fields_size: config.fields_size,
                demo_vc: false,
                seed: 42,
            },
        };
        let (credential, disclosures) = wallet.issue_sd_vc_jwt(params).unwrap();

        
        let credential_size_bytes = credential.len();
        let (_field_count, disclosures_size_bytes) = if let Value::Array(disc_arr) = disclosures {
            (disc_arr.len().saturating_sub(1), serde_json::to_string(&disc_arr).unwrap().len())
        } else {
            (0, 0)
        };
        
        let size_metrics = SizeMetrics {
            algorithm: config.wallet_type.to_string(),
            field_size: config.fields_size.to_string(),
            credential_size_bytes,
            field_count: config.fields_amount.to_usize(),
            disclosures_size_bytes,
        };
        
        results.insert(config.to_string(), size_metrics);
    }

    std::fs::create_dir_all("target/criterion").unwrap();
    
    let json_data = serde_json::to_string_pretty(&results).unwrap();
    std::fs::write("target/criterion/benchmark_size_data.json", json_data).unwrap();
    
    results
}

fn bench_size_comparison(c: &mut Criterion<WallTime>) {
    let _results = collect_size_data();

    let group = c.benchmark_group("sd_jwt_vc_sizes");
    group.finish();
}

criterion_group!(benches, 
    bench_issue_sd_jwt_vc,
    bench_verify_sd_jwt_vc, 
    bench_size_comparison
);
criterion_main!(benches);
