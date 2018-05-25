#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![cfg_attr(feature = "clippy", deny(clippy))]

#[macro_use]
extern crate serde_derive;
extern crate actix_web;
extern crate base64;
extern crate futures;
extern crate ring;
extern crate untrusted;

use std::collections::HashMap;

use actix_web::http::Method;
use actix_web::{App, FutureResponse, HttpMessage, HttpRequest, HttpResponse};
use futures::future::{self, Future};
use ring::signature;
use untrusted::Input;

// Travis public key, can be found in https://api.travis-ci.com/config
static PUB_KEY: &'static [u8] = b"-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAnQU2j9lnRtyuW36arNOc\ndzCzyKVirLUi3/aLh6UfnTVXzTnx8eHUnBn1ZeQl7Eh3J3qqdbIKl6npS27ONzCy\n3PIcfjpLPaVyGagIL8c8XgDEvB45AesC0osVP5gkXQkPUM3B2rrUmp1AZzG+Fuo0\nSAeNnS71gN63U3brL9fN/MTCXJJ6TvMt3GrcJUq5uq56qNeJTsiowK6eiiWFUSfh\ne1qapOdMFmcEs9J/R1XQ/scxbAnLcWfl8lqH/MjMdCMe0j3X2ZYMTqOHsb3cQGSS\ndMPwZGeLWV+OaxjJ7TrJ+riqMANOgqCBGpvWUnUfo046ACOx7p6u4fFc3aRiuqYK\nVQIDAQAB\n-----END PUBLIC KEY-----";

#[derive(Deserialize)]
struct Notification {}

// We need to verify that this request came from travis
// See: https://docs.travis-ci.com/user/notifications - Verifying Webhook requests
fn travis_notification(req: HttpRequest) -> FutureResponse<HttpResponse> {
    // Obtain signature and encode it using base64
    let dec_sig = {
        let sig = match req.headers().get("signature") {
            Some(sig) => sig.as_bytes(),
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
                if let Err(_) = signature::verify(
                    &signature::RSA_PKCS1_2048_8192_SHA1,
                    Input::from(PUB_KEY),
                    Input::from(&payload.as_bytes()),
                    Input::from(&dec_sig),
                ) {
                    // Request didn't come from Travis
                    eprintln!("INVALID REQUEST");
                    return Ok(HttpResponse::Forbidden().into());
                }

                println!("VALID REQUEST");
                Ok(HttpResponse::Ok().into())
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
