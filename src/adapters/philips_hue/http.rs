/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Shared HTTP functions for `PhilipsHueAdapter`
//!
//! Philips Hue bridges expose an HTTP-based API. This module aims
//! to take away some of the boilerplate involved with HTTP requests.

use hyper;
use std::io::Read;
use std::error::Error;

pub fn get(url: &str) -> Result<String, Box<Error>> {
    // return Ok(.to_owned());
    let client = hyper::Client::new();
    let mut res = try!(
        client.get(url)
            .header(hyper::header::Connection::close())
            .send());
    let mut content = String::new();
    try!(res.read_to_string(&mut content));
    Ok(content.to_owned())
}

pub fn post(url: &str, data: &str) -> Result<String, Box<Error>> {
    let client = hyper::Client::new();
    let mut res = try!(
        client.post(url)
            .body(data)
            .header(hyper::header::Connection::close())
            .send());
    let mut content = String::new();
    try!(res.read_to_string(&mut content));
    Ok(content.to_owned())
}

pub fn put(url: &str, data: &str) -> Result<String, Box<Error>> {
    let client = hyper::Client::new();
    let mut res = try!(
        client.put(url)
            .body(data)
            .header(hyper::header::Connection::close())
            .send());
    let mut content = String::new();
    try!(res.read_to_string(&mut content));
    Ok(content.to_owned())
}

#[cfg(test)]
describe! philips_hue_http {

    before_each {
        let good_url = "https://www.meethue.com/api/nupnp";
    }

    // TODO: This test fails on travis (not locally) for unknown reasons:
    // it "should GET from good addresses" {
    //     let res = get(good_url).unwrap();
    //     let is_json = res.starts_with("[{") && res.ends_with("}]");
    //     assert!(is_json);
    // }

    it "should POST to good addresses" {
        let res = post(good_url, "[]").unwrap();
        assert!(res.len() > 0);
    }

    it "should PUT to good addresses" {
        let res = put(good_url, "[]").unwrap();
        assert!(res.len() > 0);
    }
}

#[test]
#[should_panic]
fn get_should_err_on_bad_url() {
    let _ = get("http://www.meatwho.comm/").unwrap();
}
