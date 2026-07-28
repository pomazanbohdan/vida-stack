use std::{
    fs,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use vida_policy_rhai::{build_policy_engine, PolicyBundle, PolicyEngine};

const CORPUS_PATH: &str = "../../fixtures/policy-runtime/native/corpus.json";
const CORPUS_SCHEMA: &str = "vida-native-policy-baseline-v1";
const SAMPLE_COUNT: usize = 20;

#[derive(Debug, Deserialize)]
struct NativeCorpus {
    schema: String,
    families: Vec<NativeFamily>,
}

#[derive(Debug, Deserialize)]
struct NativeFamily {
    family: String,
    #[serde(flatten)]
    bundle: PolicyBundle,
    context: Value,
    expected: Value,
}

#[derive(Debug, Serialize)]
struct CanonicalOutput<'a> {
    family: &'a str,
    policy_id: &'a str,
    version: u32,
    output: &'a Value,
}

#[derive(Debug)]
struct Observation {
    output: Value,
    canonical: Vec<u8>,
    digest: String,
    elapsed: Duration,
}

fn load_corpus() -> NativeCorpus {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(CORPUS_PATH);
    let raw = fs::read_to_string(path).expect("native policy baseline corpus must be readable");
    let corpus: NativeCorpus = serde_json::from_str(&raw).expect("native corpus must be JSON");
    assert_eq!(corpus.schema, CORPUS_SCHEMA);
    assert_eq!(corpus.families.len(), 6);
    corpus
}

fn observe(engine: &PolicyEngine, family: &NativeFamily) -> Observation {
    let started = Instant::now();
    let output = engine
        .evaluate(&family.bundle.source, family.context.clone())
        .unwrap_or_else(|error| panic!("{} evaluation failed: {error}", family.family));
    let canonical = serde_json::to_vec(&CanonicalOutput {
        family: &family.family,
        policy_id: &family.bundle.policy_id,
        version: family.bundle.version,
        output: &output,
    })
    .expect("canonical native output must serialize");
    let digest = blake3::hash(&canonical).to_hex().to_string();
    Observation {
        output,
        canonical,
        digest,
        elapsed: started.elapsed(),
    }
}

fn percentile(samples: &mut [u128], numerator: u128, denominator: u128) -> u128 {
    samples.sort_unstable();
    let last = samples.len().saturating_sub(1) as u128;
    let index = (last * numerator).div_ceil(denominator) as usize;
    samples[index.min(samples.len().saturating_sub(1))]
}

fn tool_version(tool: &str) -> String {
    Command::new(tool)
        .arg("--version")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|error| format!("{tool}: unavailable ({error})"))
}

#[test]
fn native_policy_baseline_is_deterministic_across_two_corpus_runs() {
    let corpus = load_corpus();
    let engine = build_policy_engine(Default::default());
    let first = corpus
        .families
        .iter()
        .map(|family| observe(&engine, family))
        .collect::<Vec<_>>();
    let second = corpus
        .families
        .iter()
        .map(|family| observe(&engine, family))
        .collect::<Vec<_>>();
    let rustc = tool_version("rustc");
    let cargo = tool_version("cargo");

    assert_eq!(first.len(), second.len());
    for ((family, left), right) in corpus.families.iter().zip(first.iter().zip(second.iter())) {
        assert_eq!(
            left.output, family.expected,
            "{} output drift",
            family.family
        );
        assert_eq!(
            left.canonical, right.canonical,
            "{} canonical drift",
            family.family
        );
        assert_eq!(left.digest, right.digest, "{} digest drift", family.family);

        let mut samples = (0..SAMPLE_COUNT)
            .map(|_| observe(&engine, family).elapsed.as_micros())
            .collect::<Vec<_>>();
        let p50 = percentile(&mut samples, 50, 100);
        let p95 = percentile(&mut samples, 95, 100);
        let output = serde_json::to_string(&left.output).expect("output must be JSON");
        println!(
            "native_baseline artifact={CORPUS_PATH} family={} output={} digest={} p50_us={} p95_us={} rustc={rustc:?} cargo={cargo:?}",
            family.family, output, left.digest, p50, p95
        );
    }
}
