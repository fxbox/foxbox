/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/// This manages registration of the foxbox with the discovery endpoint.
/// For now it simply register itselfs every N minutes with the endpoint,
/// after trying more aggressively at first run.

extern crate get_if_addrs;
extern crate hyper;

use self::hyper::Client;
use self::hyper::header::Connection;
use self::hyper::status::StatusCode;
use self::get_if_addrs::IfAddr;
use std::io::Read;
use std::time::Duration;
use std::thread;

const DEFAULT_ENDPOINT: &'static str = "http://localhost:4242/register";
const REGISTRATION_INTERVAL: u32 = 1; // in minutes.

pub struct Registrar;

impl Registrar {
    pub fn new() -> Registrar {
        Registrar
    }

    pub fn start(&self, endpoint: Option<String>) {
        let url = match endpoint {
            Some(u) => u,
            _ => DEFAULT_ENDPOINT.to_owned()
        };

        info!("Starting registration with {}", url);
        let ip_addr = self.get_ip_addr();
        if ip_addr == None {
            // TODO: retry later, in case we're racing with the network
            // configuration.
            return;
        }

        info!("Got ip address: {}", ip_addr.clone().unwrap());
        let full_address = format!("{}?ip={}", url, ip_addr.unwrap());

        // Spawn a thread to register every REGISTRATION_INTERVAL minutes.
        thread::Builder::new().name("Registrar".to_owned())
                              .spawn(move || {
            loop {
                let client = Client::new();
                let res = client.get(&full_address)
                    .header(Connection::close())
                    .send();

                // Sanity checks, mostly to debug errors since we don't try
                // to recover from failures.
                if let Ok(mut response) = res {
                    if response.status == StatusCode::Ok {
                        let mut body = String::new();
                        if let Ok(_) = response.read_to_string(&mut body) {
                            info!("Server responded with: {}", body);
                        } else {
                            info!("Unable to read answer from {}", full_address);
                        }
                    }
                } else {
                    info!("Unable to send request to {}", full_address);
                }

                // Go to sleep.
                thread::sleep(Duration::from_secs(REGISTRATION_INTERVAL as u64 * 60))
            }
        }).unwrap();
    }

    pub fn get_ip_addr(&self) -> Option<String> {
        // Look for an ipv4 interface on eth* or wlan*.
        if let Ok(ifaces) = get_if_addrs::get_if_addrs() {
            if ifaces.is_empty() {
                error!("No IP interfaces found!");
                return None;
            }

            let mut ip_addr: Option<String> = None;

            for iface in ifaces {
                if ! (iface.name.starts_with("eth") ||
                      iface.name.starts_with("wlan") ||
                      iface.name.starts_with("en")) {
                    continue;
                }
                if let IfAddr::V4(ref v4) = iface.addr {
                    ip_addr = Some(format!("{}", v4.ip));
                }
            }

            ip_addr
        } else {
            error!("No IP interfaces found!");
            None
        }
    }
}

#[test]
fn check_ip_addr() {
    let registrar = Registrar::new();
    let ip_addr = registrar.get_ip_addr();
    assert!(ip_addr != None);
}
