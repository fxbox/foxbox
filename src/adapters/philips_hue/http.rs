/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate hyper;

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
