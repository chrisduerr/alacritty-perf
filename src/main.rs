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

mod bench;

use std::collections::HashMap;
use std::rc::Rc;

use actix_web::http::Method;
use actix_web::{App, Binary, Body, FutureResponse, HttpMessage, HttpRequest, HttpResponse};
use env_logger::Builder;
use futures::future::{self, Future};
use log::LevelFilter;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Verifier;

// Travis public key, can be found in https://api.travis-ci.com/config
static PUB_KEY: &'static [u8] = b"-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAvtjdLkS+FP+0fPC09j25\ny/PiuYDDivIT86COVedvlElk99BBYTrqNaJybxjXbIZ1Q6xFNhOY+iTcBr4E1zJu\ntizF3Xi0V9tOuP/M8Wn4Y/1lCWbQKlWrNQuqNBmhovF4K3mDCYswVbpgTmp+JQYu\nBm9QMdieZMNry5s6aiMA9aSjDlNyedvSENYo18F+NYg1J0C0JiPYTxheCb4optr1\n5xNzFKhAkuGs4XTOA5C7Q06GCKtDNf44s/CVE30KODUxBi0MCKaxiXw/yy55zxX2\n/YdGphIyQiA5iO1986ZmZCLLW8udz9uhW5jUr3Jlp9LbmphAC61bVSf4ou2YsJaN\n0QIDAQAB\n-----END PUBLIC KEY-----";

// Repository which will be allowed to bench from
static TARGET_REPO: &'static str = "jwilm/alacritty";

#[derive(Deserialize)]
pub struct Payload {
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

    // Make sure jwilm/alacritty repo is target
    {
        let target_repo = req
            .headers()
            .get("Travis-Repo-Slug")
            .and_then(|tr| tr.to_str().ok())
            .unwrap_or("");
        if target_repo != TARGET_REPO {
            warn!("Blocking invalid origin repository: '{}'", target_repo);
            return Box::new(future::ok(HttpResponse::Forbidden().into()));
        }
    }

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

                    bench::create(pl);

                    return Ok(HttpResponse::Ok().into());
                }

                // Request didn't come from Travis
                Ok(HttpResponse::Forbidden().into())
            }),
    )
}

fn results(_: HttpRequest) -> HttpResponse {
    info!("Received data request");

    let benches = bench::load();

    let json = serde_json::to_string(&benches).unwrap_or_else(|_| String::from("[]"));
    info!("Sending Response:\n{}", json);

    let body = Body::Binary(Binary::SharedString(Rc::new(json)));
    HttpResponse::Ok()
        .content_type("application/json")
        .body(body)
        .into()
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
