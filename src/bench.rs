use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::process::Command;

use chrono::offset::Utc;
use walkdir::{DirEntry, WalkDir};

use Payload;

// Directory with all the benchmarks
static RESULTS_DIR: &'static str = "./results";

#[derive(Debug, Serialize)]
struct Bench {
    name: String,
    branches: Vec<Branch>,
}

#[derive(Debug, Serialize)]
struct Branch {
    name: String,
    results: Vec<Result>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Result {
    timestamp: String,
    #[serde(flatten)]
    estimates: Estimates,
}

#[derive(Debug, Serialize, Deserialize)]
struct Estimates {
    #[serde(rename = "Mean")]
    mean: Metric,
    #[serde(rename = "Median")]
    median: Metric,
    #[serde(rename = "MedianAbsDev")]
    median_abs_dev: Metric,
    #[serde(rename = "Slope")]
    slope: Metric,
    #[serde(rename = "StdDev")]
    std_dev: Metric,
}

#[derive(Debug, Serialize, Deserialize)]
struct Metric {
    confidence_interval: ConfidenceInterval,
    point_estimate: f64,
    standard_error: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfidenceInterval {
    confidence_level: f64,
    lower_bound: f64,
    upper_bound: f64,
}

pub fn create(payload: Payload) {
    // Don't benchmark commits/PRs to branches
    if payload.branch != "master" {
        info!("Branch commit detected, skipping benchmarks");
        return;
    }

    // Create path name based on commit/pr
    let time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let (mut path, commit) = if payload.pull_request {
        (
            format!(
                "{} (#{})",
                payload.pull_request_title, payload.pull_request_number
            ),
            payload.head_commit,
        )
    } else {
        ("Master".to_owned(), payload.commit)
    };
    path = format!("{}/{}/{}-{}", RESULTS_DIR, path, time, commit);

    let command = format!("./bench.sh \"{}\" \"{}\" &", commit, path);
    info!("Running command `{}`", command);
    if let Err(err) = Command::new("bash")
        .args(&["-c", &command])
        .spawn()
        .and_then(|mut cmd| cmd.wait())
    {
        error!("Unable to start benchmark: {}", err);
    } else {
        info!("Successfully started benchmark");
    }
}

pub fn load() {
    // Hashmap for accessing benchmarks by name
    let mut benchmarks = HashMap::new();

    for entry in WalkDir::new(RESULTS_DIR)
        .min_depth(3)
        .max_depth(3)
        .into_iter()
        .filter_entry(|e| e.metadata().map(|m| m.is_file()).unwrap_or(false))
    {
        if let Ok(entry) = entry {
            info!("Processing benchmark {:?}", entry.path());

            let bench = deserialize(entry);

            if let Some(bench) = bench {
                // Merge benchmark with existing benchmarks
                if benchmarks.contains_key(&bench.name) {
                    let benches = benchmarks.get_mut(&bench.name).unwrap();
                    merge_benchmarks(benches, bench);
                } else {
                    benchmarks.insert(bench.name.clone(), bench);
                }
            }
        }
    }

    // Convert benchmarks hashmap to an array
    let mut bench_vec = Vec::new();
    for (_, bench) in benchmarks.drain() {
        bench_vec.push(bench);
    }
}

fn deserialize(entry: DirEntry) -> Option<Bench> {
    // Result's name
    let name = entry.file_name().to_string_lossy();
    info!("    NAME: {}", name);

    // Result's commit timestamp
    let parent = entry.path().parent()?;
    let commit = parent.file_name()?.to_string_lossy();
    let timestamp_len = "YYYY-MM-DDTHH:MM:SSZ".len();
    let timestamp = &commit[..timestamp_len];
    info!("    TIMESTAMP: {}", timestamp);

    // Result's branch
    let branch_parent = parent.parent()?;
    let branch = branch_parent.file_name()?.to_string_lossy();
    info!("    BRANCH: {}", branch);

    // Result's avg time
    let mut content = String::new();
    File::open(entry.path())
        .and_then(|mut f| f.read_to_string(&mut content))
        .ok()?;
    let estimates = serde_json::from_str(&content).ok()?;
    info!("    ESTIMATES:\n{:?}\n", estimates);

    // Create a benchmark with just a single data point
    Some(Bench {
        name: name.into(),
        branches: vec![Branch {
            name: branch.to_string(),
            results: vec![Result {
                timestamp: timestamp.to_owned(),
                estimates,
            }],
        }],
    })
}

// Merge a single benchmark result into existing results
fn merge_benchmarks(benches: &mut Bench, mut bench: Bench) {
    // Add single data point to existing branch
    for branch in benches.branches.iter_mut() {
        if branch.name == bench.branches[0].name {
            branch
                .results
                .push(bench.branches.remove(0).results.remove(0));
            branch.results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            return;
        }
    }

    // If branch does not exist already, add complete branch
    benches.branches.push(bench.branches.remove(0));
}
