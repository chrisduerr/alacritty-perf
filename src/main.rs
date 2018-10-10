#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", deny(clippy))]

#[macro_use]
extern crate serde_derive;
extern crate actix_web;
extern crate base64;
extern crate futures;
extern crate openssl;
extern crate serde_json;
#[macro_use]
extern crate log;
extern crate chrono;
extern crate env_logger;
extern crate walkdir;

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::process::Command;
use std::rc::Rc;

use actix_web::http::Method;
use actix_web::{App, Binary, Body, FutureResponse, HttpMessage, HttpRequest, HttpResponse};
use chrono::offset::Utc;
use env_logger::Builder;
use futures::future::{self, Future};
use log::LevelFilter;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Verifier;
use walkdir::{DirEntry, WalkDir};

// Travis public key, can be found in https://api.travis-ci.com/config
static PUB_KEY: &'static [u8] = b"-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAvtjdLkS+FP+0fPC09j25\ny/PiuYDDivIT86COVedvlElk99BBYTrqNaJybxjXbIZ1Q6xFNhOY+iTcBr4E1zJu\ntizF3Xi0V9tOuP/M8Wn4Y/1lCWbQKlWrNQuqNBmhovF4K3mDCYswVbpgTmp+JQYu\nBm9QMdieZMNry5s6aiMA9aSjDlNyedvSENYo18F+NYg1J0C0JiPYTxheCb4optr1\n5xNzFKhAkuGs4XTOA5C7Q06GCKtDNf44s/CVE30KODUxBi0MCKaxiXw/yy55zxX2\n/YdGphIyQiA5iO1986ZmZCLLW8udz9uhW5jUr3Jlp9LbmphAC61bVSf4ou2YsJaN\n0QIDAQAB\n-----END PUBLIC KEY-----";

// Directory with all the benchmarks
static RESULTS_DIR: &'static str = "./results";

#[derive(Deserialize)]
struct Payload {
    pull_request_title: String,
    pull_request_number: usize,
    pull_request: bool,
    head_commit: String,
    commit: String,
    branch: String,
}

// We need to verify that this request came from travis
// See: https://docs.travis-ci.com/user/notifications - Verifying Webhook requests
fn travis_notification(req: HttpRequest) -> FutureResponse<HttpResponse> {
    info!("Received new travis notification");

    // Obtain signature and encode it using base64
    let dec_sig = {
        let sig = match req.headers().get("Signature") {
            Some(sig) => sig,
            None => {
                warn!("Signature header missing");
                return Box::new(future::ok(HttpResponse::Forbidden().into()));
            }
        };
        match base64::decode(sig) {
            Ok(dec_sig) => dec_sig,
            Err(_) => {
                warn!("Unable to decode signature as base64");
                return Box::new(future::ok(HttpResponse::Forbidden().into()));
            }
        }
    };
    info!("Signature successfully decoded");

    Box::new(
        req.urlencoded::<HashMap<String, String>>()
            .from_err()
            .and_then(move |body| {
                // Get the request payload
                let payload = body.get("payload").map(|pl| pl.as_str()).unwrap_or("");

                // Verify the payload
                // Unwraps are safe becase public key is hardcoded
                let pkey = PKey::from_rsa(Rsa::public_key_from_pem(PUB_KEY).unwrap()).unwrap();
                let mut verifier = Verifier::new(MessageDigest::sha1(), &pkey).unwrap();
                if let Err(_) = verifier.update(&payload.as_bytes()) {
                    warn!("Unable to update verifier with payload");
                    return Ok(HttpResponse::Forbidden().into());
                }
                if let Ok(true) = verifier.verify(&dec_sig) {
                    info!("Request verification successful");
                    let pl: Payload = match serde_json::from_str(&payload) {
                        Ok(pl) => pl,
                        Err(_) => {
                            warn!("Skipping payload with invalid format");
                            return Ok(HttpResponse::Forbidden().into());
                        }
                    };

                    // Don't benchmark commits/PRs to branches
                    if pl.branch != "master" {
                        info!("Branch commit detected, skipping benchmarks");
                        return Ok(HttpResponse::Ok().into());
                    }

                    // Create path name based on commit/pr
                    // Following https://www.w3.org/TR/NOTE-datetime (2018-12-31T12:45:45Z)
                    let time = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
                    let (mut path, commit) = if pl.pull_request {
                        (
                            format!("{} (#{})", pl.pull_request_title, pl.pull_request_number),
                            pl.head_commit,
                        )
                    } else {
                        ("Master".to_owned(), pl.commit)
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

                    return Ok(HttpResponse::Ok().into());
                }

                // Request didn't come from Travis
                Ok(HttpResponse::Forbidden().into())
            }),
    )
}

#[derive(Serialize)]
struct Bench {
    name: String,
    branches: Vec<Branch>,
}

#[derive(Serialize)]
struct Branch {
    name: String,
    results: Vec<Result>,
}

#[derive(Serialize)]
struct Result {
    timestamp: String,
    mean: u128,
}

fn results(_: HttpRequest) -> HttpResponse {
    info!("Received data request");

    // Hashmap for accessing benchmarks by name
    let mut benchmarks = HashMap::new();

    for entry in WalkDir::new(RESULTS_DIR)
        .min_depth(3)
        .max_depth(3)
        .into_iter()
        .filter_entry(|e| e.metadata().map(|m| m.is_file()).unwrap_or(false))
    {
        if let Ok(entry) = entry {
            info!("Processing benchmark '{:?}'", entry.path());
            let bench = entry_to_result(entry);
            if let Some(bench) = bench {
                info!(
                    "    NAME: {}\n    TIMESTAMP: {}\n    MEAN: {}",
                    bench.name,
                    bench.branches[0].results[0].timestamp,
                    bench.branches[0].results[0].mean
                );
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

    let json = serde_json::to_string(&bench_vec).unwrap_or_else(|_| String::from("[]"));
    info!("Sending Response:\n{}", json);
    let body = Body::Binary(Binary::SharedString(Rc::new(json)));
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
        .into()
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

fn entry_to_result(entry: DirEntry) -> Option<Bench> {
    // Result's name
    let name = entry.file_name().to_string_lossy();

    // Result's commit timestamp
    let parent = entry.path().parent()?;
    let commit = parent.file_name()?.to_string_lossy();
    let timestamp_len = "YYYY-MM-DDTHH:MM:SSZ".len();
    let timestamp = &commit[..timestamp_len];

    // Result's branch
    let branch_parent = parent.parent()?;
    let branch = branch_parent.file_name()?.to_string_lossy();

    // Result's mean time
    let mut content = String::new();
    File::open(entry.path())
        .and_then(|mut f| f.read_to_string(&mut content))
        .ok()?;
    let mean = content.split(";").collect::<Vec<&str>>()[0];
    let mean = u128::from_str_radix(mean, 10).ok()?;

    // Create a benchmark with just a single data point
    Some(Bench {
        name: name.into(),
        branches: vec![Branch {
            name: branch.to_string(),
            results: vec![Result {
                timestamp: timestamp.to_owned(),
                mean,
            }],
        }],
    })
}

fn main() {
    Builder::new().filter_level(LevelFilter::Info).init();
    info!("Logger started successfully");

    info!("Starting server...");
    actix_web::server::new(|| {
        App::new()
            .resource("/notify", |r| {
                r.method(Method::POST).with(travis_notification)
            })
            .resource("/data", |r| r.method(Method::GET).with(results))
    })
    .bind("127.0.0.1:8080")
    .expect("Unable to bind to address")
    .run();
}
