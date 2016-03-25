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
use serde_json;
use std::error::Error;
use std::io::Read;
use std::time::Duration;
use std::thread;
use tls::CertificateManager;
use tunnel_controller:: { Tunnel };

const REGISTRATION_INTERVAL_IN_MINUTES: u32 = 1;

pub struct Registrar {
    box_name: String,
}

#[derive(Serialize, Debug)]
struct RegistrationRequest {
    message: String,
    client: String,

    // Included for backwards compat, to be removed
    local_ip: String,
    tunnel_url: Option<String>,
}

impl Registrar {
    pub fn new(box_name: String) -> Registrar {
        Registrar {
            box_name: box_name
        }
    }

    pub fn start(&self, endpoint_url: String,
                 iface: Option<String>,
                 domain: String,
                 tunnel: &Option<Tunnel>,
                 box_port: u16,
                 dns_api_endpoint: String,
                 certificate_manager: CertificateManager) {
        info!("Starting registration with {}", endpoint_url);
        let endpoint_url = format!("{}/register", endpoint_url);

        let ip_addr = self.get_ip_addr(&iface);
        if ip_addr == None {
            // TODO: retry later, in case we're racing with the network
            // configuration.
            return;
        }

        info!("Got ip address: {}", ip_addr.clone().unwrap());

        let certificate_record_result = certificate_manager
                                            .get_or_generate_self_signed_certificate(&self.box_name);

        let (domain, client_fingerprint) = if let Ok(certificate_record) = certificate_record_result {
            let self_signed_cert_fingerprint = certificate_record.get_certificate_fingerprint();
            let fingerprint_domain = format!("{}.{}", self_signed_cert_fingerprint, domain);

            info!("Using {}", fingerprint_domain);
            (fingerprint_domain, self_signed_cert_fingerprint)
        } else {
            panic!("Could not get or generate self signed certificate - registration will always fail: {}", certificate_record_result.err().unwrap().description());
        };

        let tunnel_url = if let Some(ref tunnel) = *tunnel {
            tunnel.get_remote_hostname()
        } else {
            None
        };

        let message = json!({
            local_address: format!("local.{}:{}", domain, box_port),
            tunnel_url: tunnel_url
        });

        let body = match serde_json::to_string(&RegistrationRequest {
            message: message,
            client: client_fingerprint,
            local_ip: ip_addr.clone().unwrap(),
            tunnel_url: tunnel_url,
        }) {
            Ok(body) => body,
            Err(_) => {
                error!("Serialization error");
                return;
            }
        };

        let box_hostname = self.box_name.clone();

        // Spawn a thread to register every REGISTRATION_INTERVAL_IN_MINUTES.
        thread::Builder::new().name("Registrar".to_owned())
                              .spawn(move || {
            loop {
                let client = Client::new();
                let res = client.post(&endpoint_url)
                    .header(Connection::close())
                    .body(&body)
                    .send();

                // Sanity checks, mostly to debug errors since we don't try
                // to recover from failures.
                if let Ok(mut response) = res {
                    if response.status == StatusCode::Ok {
                        let mut body = String::new();
                        if let Ok(_) = response.read_to_string(&mut body) {
                            info!("Server responded with: {}", body);
                        } else {
                            info!("Unable to read answer from {}", endpoint_url);
                        }
                    }
                } else {
                    info!("Unable to send request to {}", endpoint_url);
                }

                // Create entry for local DNS
                certificate_manager.register_dns_record(
                    "A",
                    &format!("local.{}", domain),
                    &ip_addr.clone().unwrap(),
                    &dns_api_endpoint.clone(),
                    &box_hostname
                ).unwrap();

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
    fn get_ip_addr_from_ifaces(&self, ifaces: &[Interface],
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
        let registrar = Registrar::new("foxbox.local".to_owned());
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

        it "should return default IPv4" {
            let ip_addr = registrar.get_ip_addr_from_ifaces(&interfaces, &None)
                .unwrap();
            assert_eq!(ip_addr, "192.168.0.4");
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
