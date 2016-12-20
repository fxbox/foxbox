// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use url::{ParseError, Url};

use pagekite::{PageKite, InitFlags, LOG_NORMAL};

pub struct Tunnel {
    config: TunnelConfig,
    pub pagekite: Option<PageKite>,
}

#[derive(Clone, Debug)]
pub struct TunnelConfig {
    /// The socket address that the box connects to to establish the tunnel.
    tunnel_url: Url,
    tunnel_secret: String,
    local_http_port: u16,
    local_ws_port: u16,
    remote_name: String,
}

impl TunnelConfig {
    pub fn new(tunnel_url: &str,
               tunnel_secret: &str,
               local_http_port: u16,
               local_ws_port: u16,
               remote_name: &str)
               -> Self {

        fn invalid_url() {
            error!("Could not parse tunnel url.
                        Try something like knilxof.org:443");
        }

        let tunnel_url = match Url::parse(tunnel_url) {
            Ok(url) => {
                // If we have no domain, reparse with http:// in front.
                if url.domain().is_none() {
                    match Url::parse(&format!("http://{}", tunnel_url)) {
                        Ok(url) => url,
                        Err(err) => {
                            invalid_url();
                            panic!(err);
                        }
                    }
                } else {
                    url
                }
            }
            Err(err) => {
                invalid_url();
                panic!(err);
            }
        };

        TunnelConfig {
            tunnel_url: tunnel_url,
            tunnel_secret: String::from(tunnel_secret),
            local_http_port: local_http_port,
            local_ws_port: local_ws_port,
            remote_name: String::from(remote_name),
        }
    }
}

impl Tunnel {
    /// Create a new Tunnel object containing the necessary
    /// configuration
    pub fn new(config: TunnelConfig) -> Tunnel {
        Tunnel {
            config: config,
            pagekite: None,
        }
    }

    /// Start the Tunnel process if it has not already been started
    pub fn start(&mut self) -> Result<(), ()> {
        if self.pagekite.is_some() {
            // Already started
            Ok(())
        } else {
            // Describes how to configure pagekite.
            // pagekite requires a user (remote_name) and a shared secret to be able
            // to connect us with the bridge. For the first prototype we will have a
            // secret common to all boxes, but in the end we will need a secret per
            // box. Unfortunately pagekite does not provide a way to add a new
            // domain/secret pair while the bridge is running, but it provides the
            // possibility to delegate the authentication to a dynamic DNS server.
            // XXX We will move to DNS authentication after the first prototype if
            // we keep using pagekite.
            // https://github.com/fxbox/foxbox/issues/177#issuecomment-194778308
            self.pagekite = PageKite::init(Some("foxbox"),
                                           2, // max kites: one for https and one for websocket.
                                           1, // max frontends
                                           10, // max connections.
                                           None, // dyndns url
                                           &[InitFlags::WithIpv4, InitFlags::WithIpv6],
                                           &LOG_NORMAL);
            if let Some(ref pagekite) = self.pagekite {
                let tunnel_domain = match self.config.tunnel_url.domain() {
                    Some(domain) => domain,
                    None => {
                        panic!("No tunnel domain found. Cannot start tunneling");
                    }
                };

                let tunnel_port = match self.config.tunnel_url.port() {
                    Some(port) => port,
                    None => {
                        panic!("No tunnel port found. Cannot start tunneling");
                    }
                };
                info!("Setting up tunnel for remote nanamed {}",
                      self.config.remote_name);
                pagekite.lookup_and_add_frontend(tunnel_domain, tunnel_port as i32, true);
                info!("Adding kite for https on port {}",
                      self.config.local_http_port);
                pagekite.add_kite("https",
                                  &self.config.remote_name,
                                  tunnel_port as i32,
                                  &self.config.tunnel_secret,
                                  "localhost",
                                  self.config.local_http_port as i32);
                info!("Adding kite for websocket on port {}",
                      self.config.local_ws_port);
                pagekite.add_kite("websocket",
                                  &self.config.remote_name,
                                  tunnel_port as i32,
                                  &self.config.tunnel_secret,
                                  "localhost",
                                  self.config.local_ws_port as i32);
                pagekite.thread_start();
                Ok(())
            } else {
                Err(())
            }
        }
    }

    /// Stop the tunnel process if it is runnnig
    pub fn stop(&mut self) -> Result<(), ()> {
        if let Some(ref pagekite) = self.pagekite {
            pagekite.thread_stop();
        }
        self.pagekite = None;
        Ok(())
    }

    pub fn get_frontend_name(&self) -> Option<String> {
        match self.config.tunnel_url.host() {
            Some(host) => Some(host.to_string()),
            None => None,
        }
    }
}

#[test]
fn test_tunnel_url() {
    let config = TunnelConfig::new("knilxof.org:443", "secret", 80, 80, "remote");
    assert_eq!(config.tunnel_url.domain().unwrap(), "knilxof.org");
    let config = TunnelConfig::new("http://knilxof.org:443", "secret", 80, 80, "remote");
    assert_eq!(config.tunnel_url.domain().unwrap(), "knilxof.org");
}
