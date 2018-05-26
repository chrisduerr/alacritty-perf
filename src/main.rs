#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", deny(clippy))]

#[macro_use]
extern crate serde_derive;
extern crate actix_web;
extern crate base64;
extern crate futures;
extern crate openssl;

use std::collections::HashMap;
use std::process::Command;

use actix_web::http::Method;
use actix_web::{App, FutureResponse, HttpMessage, HttpRequest, HttpResponse};
use futures::future::{self, Future};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Verifier;

// Travis public key, can be found in https://api.travis-ci.com/config
static PUB_KEY: &'static [u8] = b"-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAvtjdLkS+FP+0fPC09j25\ny/PiuYDDivIT86COVedvlElk99BBYTrqNaJybxjXbIZ1Q6xFNhOY+iTcBr4E1zJu\ntizF3Xi0V9tOuP/M8Wn4Y/1lCWbQKlWrNQuqNBmhovF4K3mDCYswVbpgTmp+JQYu\nBm9QMdieZMNry5s6aiMA9aSjDlNyedvSENYo18F+NYg1J0C0JiPYTxheCb4optr1\n5xNzFKhAkuGs4XTOA5C7Q06GCKtDNf44s/CVE30KODUxBi0MCKaxiXw/yy55zxX2\n/YdGphIyQiA5iO1986ZmZCLLW8udz9uhW5jUr3Jlp9LbmphAC61bVSf4ou2YsJaN\n0QIDAQAB\n-----END PUBLIC KEY-----";

#[derive(Deserialize)]
struct Notification {}

// We need to verify that this request came from travis
// See: https://docs.travis-ci.com/user/notifications - Verifying Webhook requests
fn travis_notification(req: HttpRequest) -> FutureResponse<HttpResponse> {
    // Obtain signature and encode it using base64
    let dec_sig = {
        let sig = match req.headers().get("Signature") {
            Some(sig) => sig,
            None => return Box::new(future::ok(HttpResponse::Forbidden().into())),
        };
        match base64::decode(sig) {
            Ok(dec_sig) => dec_sig,
            Err(_) => return Box::new(future::ok(HttpResponse::Forbidden().into())),
        }
    };

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
                    return Ok(HttpResponse::Forbidden().into());
                }
                if let Ok(true) = verifier.verify(&dec_sig) {
                    Command::new("bash")
                        .args(&["-c", "./headless-bench.sh &"])
                        .spawn()
                        .expect("Unable to start benchmark");
                    return Ok(HttpResponse::Ok().into());
                }

                // Request didn't come from Travis
                Ok(HttpResponse::Forbidden().into())
            }),
    )
}

fn main() {
    actix_web::server::new(|| {
        App::new().resource("/notify", |r| {
            r.method(Method::POST).with(travis_notification)
        })
    }).bind("127.0.0.1:8080")
        .expect("Unable to bind to address")
        .run();
}
