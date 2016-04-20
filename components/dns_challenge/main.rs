/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![allow(dead_code)]

// Needed to derive `Serialize` on ServiceProperties
#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]

// Make linter fail for every warning
#![plugin(clippy)]
#![deny(clippy)]
// Needed for many #[derive(...)] macros
#![allow(used_underscore_binding)]

#![cfg_attr(test, feature(const_fn))] // Dependency of stainless
#![cfg_attr(test, plugin(stainless))] // Test runner

#![feature(associated_consts)]

extern crate tls;

use std::env::{ args, var };
use std::path::PathBuf;

use tls::*;

fn main() {
    let mut arguments = args();
    arguments.next();

    let hostname = arguments.next();
    if hostname.is_none() {
        panic!("No hostname set should be 1st argument");
    }

    println!("Hostname: {:?}", hostname);

    let challenge_value = arguments.next();
    if challenge_value.is_none() {
        panic!("No challenge value set, should be 2nd argument");
    }

    println!("Challenge value: {:?}", challenge_value);

    let certificate_directory_result = var("CERTIFICATE_DIRECTORY");
    let dns_api_result = var("DNS_API_ENDPOINT");

    if certificate_directory_result.is_err() {
        panic!("The CERTIFICATE_DIRECTORY environment variable should be set");
    }

    if dns_api_result.is_err() {
        panic!("The DNS_API_ENDPOINT environment variable should be set");
    }


    let dns_api = dns_api_result.unwrap();
    let certificate_directory = certificate_directory_result.unwrap();
    println!("Using certificate directory: {:?}", certificate_directory);
    println!("Using DNS api endpoint: {:?}", dns_api);

    let certificate_manager = CertificateManager::new(PathBuf::from(&certificate_directory), Box::new(SniSslContextProvider::new()));

    let box_cert = certificate_manager.get_box_certificate().unwrap();
    println!("Registering DNS record");
    register_dns_record(box_cert, &DnsRecord {
            record_type: "TXT",
            name: &format!("_acme-challenge.{}", hostname.unwrap()),
            value: &challenge_value.unwrap()
        },
        &dns_api
    ).unwrap();
}
