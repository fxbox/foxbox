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
use self::get_if_addrs::{ IfAddr, Interface };
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

    pub fn start(&self, endpoint: Option<String>, iface: Option<String>) {
        let url = match endpoint {
            Some(u) => u,
            _ => DEFAULT_ENDPOINT.to_owned()
        };

        info!("Starting registration with {}", url);
        let ip_addr = self.get_ip_addr(&iface);
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

    /// return the host IP address of the first valid interface.
    /// want_iface is an options string for the interface you want.
    pub fn get_ip_addr(&self, want_iface: &Option<String>) -> Option<String> {
        // Look for an ipv4 interface on eth* or wlan*.
        if let Ok(ifaces) = get_if_addrs::get_if_addrs() {
            if ifaces.is_empty() {
                error!("No IP interfaces found!");
                return None;
            }

            self.get_ip_addr_from_ifaces(&ifaces, want_iface)
        } else {
            error!("No IP interfaces found!");
            None
        }
    }

    /// This is a private function that to which we pass the ifaces
    /// This is so that we can shim get_if_addrs() in tests with a
    /// pre-set list of interfaces.
    fn get_ip_addr_from_ifaces(&self, ifaces: &Vec<Interface>,
                               want_iface: &Option<String>) -> Option<String> {

        let mut ip_addr: Option<String> = None;

        for iface in ifaces {
            match want_iface.as_ref() {
                None =>
                // Whitelist known good iface
                    if !(iface.name.starts_with("eth") ||
                         iface.name.starts_with("wlan") ||
                         iface.name.starts_with("en") ||
                         iface.name.starts_with("em") ||
                         iface.name.starts_with("wlp3s")) {
                        continue;
                    },
                Some(iface_name) =>
                    if &iface.name != iface_name {
                        continue;
                    }
            }
            if let IfAddr::V4(ref v4) = iface.addr {
                ip_addr = Some(format!("{}", v4.ip));
                break;
            }
        }

        if ip_addr.is_none() {
            error!("No IP interfaces found!");
        }
        ip_addr
    }

}

#[test]
fn check_ip_addr() {
    use self::get_if_addrs::*;
    use std::net::Ipv4Addr;

    let registrar = Registrar::new();
    let ip_addr = registrar.get_ip_addr(&None);
    assert!(ip_addr != None);

    // XXX add IPv6 whenever
    let ifaces1: Vec<Interface> = vec![
        Interface {
            name: "lo".to_owned(),
            addr: IfAddr::V4(Ifv4Addr { ip: Ipv4Addr::new(127,0,0,1),
                                        netmask: Ipv4Addr::new(255,0,0,0),
                                        broadcast: None })
        },
        Interface {
            name: "docker0".to_owned(),
            addr: IfAddr::V4(Ifv4Addr { ip: Ipv4Addr::new(172,18,1,42),
                                        netmask: Ipv4Addr::new(255,255,255,0),
                                        broadcast: None })
        },
        Interface {
            name: "eth0".to_owned(),
            addr: IfAddr::V4(Ifv4Addr { ip: Ipv4Addr::new(192,168,0,4),
                                        netmask: Ipv4Addr::new(255,255,255,0),
                                        broadcast: Some(
                                            Ipv4Addr::new(192,160,0,255)) })
        },
        Interface {
            name: "wlan0".to_owned(),
            addr: IfAddr::V4(Ifv4Addr { ip: Ipv4Addr::new(192,168,0,14),
                                        netmask: Ipv4Addr::new(255,255,255,0),
                                        broadcast: Some(
                                            Ipv4Addr::new(192,160,0,255)) })
        }
        ];

    // we should get eth0 address.
    let ip_addr = registrar.get_ip_addr_from_ifaces(&ifaces1, &None);
    assert!(ip_addr == Some("192.168.0.4".to_owned()));
    // we should get eth0 address as well.
    let ip_addr = registrar.get_ip_addr_from_ifaces(&ifaces1,
                                                    &Some("eth0".to_owned()));
    assert!(ip_addr == Some("192.168.0.4".to_owned()));
    // we should get wlan0 address.
    let ip_addr = registrar.get_ip_addr_from_ifaces(&ifaces1,
                                                    &Some("wlan0".to_owned()));
    assert!(ip_addr == Some("192.168.0.14".to_owned()));
    // we should get docker0 address.
    let ip_addr = registrar.get_ip_addr_from_ifaces(&ifaces1,
                                                    &Some("docker0".to_owned()));
    assert!(ip_addr == Some("172.18.1.42".to_owned()));
}
