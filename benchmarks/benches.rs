use std::{
    collections::HashMap, env, path::PathBuf, process::Command, sync::Mutex, time::Duration,
};

use criterion::{measurement::Measurement, Criterion};

pub struct Instructions;
struct InstructionsFormatter;

impl Measurement for Instructions {
    type Intermediate = ();

    type Value = u64;

    fn start(&self) -> Self::Intermediate {
        panic!("Instruction measurements must be custom calculated");
    }

    fn end(&self, _i: Self::Intermediate) -> Self::Value {
        panic!("Instruction measurements must be custom calculated");
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        v1 + v2
    }

    fn zero(&self) -> Self::Value {
        0
    }

    fn to_f64(&self, value: &Self::Value) -> f64 {
        *value as f64
    }

    fn formatter(&self) -> &dyn criterion::measurement::ValueFormatter {
        &InstructionsFormatter
    }
}

impl criterion::measurement::ValueFormatter for InstructionsFormatter {
    fn scale_values(&self, typical_value: f64, values: &mut [f64]) -> &'static str {
        if typical_value < 10_000.0 {
            return "Instructions";
        }
        if typical_value < 10_000_000.0 {
            for v in values {
                *v /= 1_000.0;
            }
            return "K Instructions";
        }
        if typical_value < 10_000_000_000.0 {
            for v in values {
                *v /= 1_000_000.0;
            }
            return "M Instructions";
        }
        if typical_value < 10_000_000_000_000.0 {
            for v in values {
                *v /= 1_000_000_000.0;
            }
            return "B Instructions";
        }
        for v in values {
            *v /= 1_000_000_000_000.0;
        }
        "T Instructions"
    }

    fn scale_throughputs(
        &self,
        typical_value: f64,
        _throughput: &criterion::Throughput,
        values: &mut [f64],
    ) -> &'static str {
        self.scale_values(typical_value, values)
    }

    fn scale_for_machines(&self, _values: &mut [f64]) -> &'static str {
        "Instructions"
    }
}

fn bench_dir() -> PathBuf {
    PathBuf::new().join(env::var("CARGO_MANIFEST_DIR").unwrap())
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct ExecutionArguments {
    method: String,
}

impl ExecutionArguments {
    fn new(method: &str) -> Self {
        Self {
            method: method.to_string(),
        }
    }
}

lazy_static::lazy_static! {
    static ref CACHED_RESULTS: Mutex<HashMap<ExecutionArguments, u64>> = Mutex::new(HashMap::new());
}

fn execution_instructions(arguments: ExecutionArguments) -> u64 {
    // Since execution will be deterministic and Criterion won't let us run it
    // only once, we'll cache the result of a given execution and immediatelly
    // return the same value on subsequent runs.
    if let Some(&result) = CACHED_RESULTS.lock().unwrap().get(&arguments) {
        return result;
    }

    let output = Command::new("bash")
        .current_dir(bench_dir())
        .args(vec!["run-benchmark.sh", &arguments.method])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(output.status.success(), "{stdout}\n{stderr}");
    for line in stderr.split("\n") {
        println!("{}", line);
    }

    // Convert result formatted as "(1_000_000 : nat64)" to u64.
    let result = stdout
        .trim()
        .strip_prefix('(')
        .unwrap()
        .strip_suffix(" : nat64)")
        .unwrap()
        .chars()
        .filter(|&c| c != '_')
        .collect::<String>()
        .parse()
        .unwrap();
    CACHED_RESULTS.lock().unwrap().insert(arguments, result);
    result
}

fn bench_function(c: &mut Criterion<Instructions>, method: &str) {
    c.bench_function(method, |b| {
        b.iter_custom(|iters| {
            // Each run will have the same result, so just do it once and
            // multiply by the number of iterations.
            iters * execution_instructions(ExecutionArguments::new(method))
        })
    });
}

pub fn criterion_benchmark(c: &mut Criterion<Instructions>) {
    bench_function(c, "insert_block_headers");
    bench_function(c, "insert_block_headers_multiple_times");
    bench_function(c, "insert_300_blocks");
    bench_function(c, "get_metrics");
}

fn benches() {
    let mut c = Criterion::default()
        .with_measurement(Instructions)
        // 10 is the smallest sample size allowed.
        .sample_size(10)
        // Should limit us to one warm-up run.
        .warm_up_time(Duration::from_millis(1))
        // Large enough to suppress warnings about not being able to complete 10
        // samples in time.
        .measurement_time(Duration::from_secs(500))
        .configure_from_args();
    criterion_benchmark(&mut c);
}

fn main() {
    assert!(Command::new("bash")
        .args(["../scripts/build-canister.sh", "benchmarks"])
        .current_dir(bench_dir())
        .status()
        .unwrap()
        .success());

    benches();
}
