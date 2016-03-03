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

const REGISTRATION_INTERVAL_IN_MINUTES: u32 = 1;

pub struct Registrar;

impl Registrar {
    pub fn new() -> Registrar {
        Registrar
    }

    pub fn start(&self, endpoint_url: String, iface: Option<String>) {
        info!("Starting registration with {}", endpoint_url);
        let ip_addr = self.get_ip_addr(&iface);
        if ip_addr == None {
            // TODO: retry later, in case we're racing with the network
            // configuration.
            return;
        }

        info!("Got ip address: {}", ip_addr.clone().unwrap());
        let full_address = format!("{}?ip={}", endpoint_url, ip_addr.unwrap());

        // Spawn a thread to register every REGISTRATION_INTERVAL_IN_MINUTES.
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
                thread::sleep(Duration::from_secs(REGISTRATION_INTERVAL_IN_MINUTES as u64 * 60))
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
        let mut ipv6_addr: Option<String> = None;

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
            } else if ipv6_addr.is_none() {
                if let IfAddr::V6(ref v6) = iface.addr {
                    ipv6_addr = Some(format!("{}", v6.ip));
                }
            }
        }

        if ip_addr.is_none() {
            if ipv6_addr.is_none() {
                error!("No IP interfaces found!");
            } else {
                ip_addr = ipv6_addr;
            }
        }
        ip_addr
    }

}

#[cfg(test)]
describe! registrar {

    before_each {
        let registrar = Registrar::new();
    }

    it "should return an IP address when a machine has network interfaces" {
        use regex::Regex;

        let ip = registrar.get_ip_addr(&None).unwrap();
        let ipv4_regex = Regex::new(r"^(\d{1,3}\.){3}\d{1,3}$").unwrap();
        assert!(ipv4_regex.is_match(ip.as_str()));
    }

    describe! ipv4 {
        before_each {
            use super::super::get_if_addrs::*;
            use std::net::Ipv4Addr;

            let interfaces: Vec<Interface> = vec![
                Interface {
                    name: "lo".to_owned(),
                    addr: IfAddr::V4(Ifv4Addr {
                        ip: Ipv4Addr::new(127,0,0,1),
                        netmask: Ipv4Addr::new(255,0,0,0),
                        broadcast: None
                    })
                },
                Interface {
                    name: "docker0".to_owned(),
                    addr: IfAddr::V4(Ifv4Addr {
                        ip: Ipv4Addr::new(172,18,1,42),
                        netmask: Ipv4Addr::new(255,255,255,0),
                        broadcast: None
                    })
                },
                Interface {
                    name: "eth0".to_owned(),
                    addr: IfAddr::V4(Ifv4Addr {
                        ip: Ipv4Addr::new(192,168,0,4),
                        netmask: Ipv4Addr::new(255,255,255,0),
                        broadcast: Some(Ipv4Addr::new(192,160,0,255))
                    })
                },
                Interface {
                    name: "wlan0".to_owned(),
                    addr: IfAddr::V4(Ifv4Addr {
                        ip: Ipv4Addr::new(192,168,0,14),
                        netmask: Ipv4Addr::new(255,255,255,0),
                        broadcast: Some(Ipv4Addr::new(192,160,0,255))
                    })
                }
            ];
        }

        it "should default to eth0" {
            let ip_addr = registrar.get_ip_addr_from_ifaces(&interfaces, &None).unwrap();
            assert_eq!(ip_addr, "192.168.0.4");
        }

        it "should retrieve address from eth0" {
            let ip_addr = registrar.get_ip_addr_from_ifaces(&interfaces, &Some("eth0".to_owned()))
                .unwrap();
            assert_eq!(ip_addr, "192.168.0.4");
        }

        it "should retrieve address from wlan0" {
            let ip_addr = registrar.get_ip_addr_from_ifaces(&interfaces, &Some("wlan0".to_owned()))
                .unwrap();
            assert_eq!(ip_addr, "192.168.0.14");
        }

        it "should retrieve address from docker0" {
            let ip_addr = registrar.get_ip_addr_from_ifaces(&interfaces, &Some("docker0".to_owned())).unwrap();
            assert_eq!(ip_addr, "172.18.1.42");
        }
    }

    describe! ipv6 {
        before_each {
            use super::super::get_if_addrs::*;
            use std::net::{ Ipv4Addr, Ipv6Addr };

            let interfaces: Vec<Interface> = vec![
                Interface {
                    name: "eth0".to_owned(),
                    addr: IfAddr::V6(Ifv6Addr {
                        ip: Ipv6Addr::new(0,0,0,0,0,0xffff,0xc0a8,0x4),
                        netmask: Ipv6Addr::new(0xffff,0xffff,0xffff,0xffff,
                                               0xffff,0xffff,0xffff,0xffff),
                        broadcast: None
                    })
                },
                Interface {
                    name: "eth0".to_owned(),
                    addr: IfAddr::V4(Ifv4Addr {
                        ip: Ipv4Addr::new(192,168,0,4),
                        netmask: Ipv4Addr::new(255,255,255,0),
                        broadcast: Some(Ipv4Addr::new(192,160,0,255))
                    })
                },
                Interface {
                    name: "eth1".to_owned(),
                    addr: IfAddr::V6(Ifv6Addr {
                        ip: Ipv6Addr::new(0,0,0,0,0,0xffff,0xc0a8,0x4),
                        netmask: Ipv6Addr::new(0xffff,0xffff,0xffff,0xffff,
                                               0xffff,0xffff,0xffff,0xffff),
                        broadcast: None
                    })
                }
            ];
        }

        it "should return IPv6" {
            let ip_addr = registrar.get_ip_addr_from_ifaces(&interfaces, &Some("eth1".to_owned()))
                .unwrap();
            assert_eq!(ip_addr, "::ffff:192.168.0.4");
        }

        it "should return IPv4 if both are specified" {
            let ip_addr = registrar.get_ip_addr_from_ifaces(&interfaces, &Some("eth0".to_owned()))
                .unwrap();
            assert_eq!(ip_addr, "192.168.0.4");
        }

        it "should return IPv4 and eth0 by default" {
            let ip_addr = registrar.get_ip_addr_from_ifaces(&interfaces, &None).unwrap();
            assert_eq!(ip_addr, "192.168.0.4");
        }
    }
}
