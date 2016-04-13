/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Assumes Unix
use std::io::prelude::*;
use std::process::{ Child, Command };
use std::io::Result;
use url::Url;
use managed_process::ManagedProcess;

pub type TunnelProcess = ManagedProcess;

pub struct Tunnel {
    config: TunnelConfig,
    pub tunnel_process: Option<TunnelProcess>
}

#[derive(Clone, Debug)]
pub struct TunnelConfig {
    /// The socket address that the box connects to to establish the tunnel.
    tunnel_url: String,
    tunnel_secret: String,
    local_http_port: u16,
    local_ws_port: u16,
    remote_name: String
}

impl TunnelConfig {
    pub fn new(tunnel_url: String,
               tunnel_secret: String,
               local_http_port: u16,
               local_ws_port: u16,
               remote_name: String) -> Self {
        TunnelConfig {
            tunnel_url: tunnel_url,
            tunnel_secret: tunnel_secret,
            local_http_port: local_http_port,
            local_ws_port: local_ws_port,
            remote_name: remote_name
        }
    }

    /// Describes how to spawn the pagekite process.
    /// pagekite requires a user (remote_name) and a shared secret to be able
    /// to connect us with the bridge. For the first prototype we will have a
    /// secret common to all boxes, but in the end we will need a secret per
    /// box. Unfortunately pagekite does not provide a way to add a new
    /// domain/secret pair while the bridge is running, but it provides the
    /// possibility to delegate the authentication to a dynamic DNS server.
    /// XXX We will move to DNS authentication after the first prototype if
    /// we keep using pagekite.
    /// https://github.com/fxbox/foxbox/issues/177#issuecomment-194778308
    pub fn spawn(&self) -> Result<Child> {
        Command::new("pagekite.py")
                .arg(format!("--frontend={}", self.tunnel_url))
                // XXX remove http service once we support https
                .arg(format!("--service_on=http,https:{}:localhost:{}:{}",
                             self.remote_name,
                             self.local_http_port,
                             self.tunnel_secret))
                .arg(format!("--service_on=websocket:{}:localhost:{}:{}",
                             self.remote_name,
                             self.local_ws_port,
                             self.tunnel_secret))
                .spawn()
    }
}

impl Tunnel {
    /// Create a new Tunnel object containing the necessary
    /// configuration
    pub fn new(config: TunnelConfig) -> Tunnel {
        Tunnel {
            config: config,
            tunnel_process: None
        }
    }

    /// Start the Tunnel process if it has not already been started
    pub fn start(&mut self) -> Result<()> {
        if let Some(_) = self.tunnel_process {
            // Already started
            Ok(())
        } else {
            let tunnel_config = self.config.clone();

            self.tunnel_process = Some(ManagedProcess::start(move || {
                tunnel_config.spawn()
            }).unwrap());

            Ok(())
        }
    }

    /// Stop the tunnel process if it is runnnig
    pub fn stop(&mut self) -> Result<()> {
        match self.tunnel_process.take() {
            None => Ok(()),
            Some(process) => process.shutdown()
        }
    }

    pub fn get_frontend_name(&self) -> Option<String> {
        if self.tunnel_process.is_none() {
            None
        } else {
            match Url::parse(&self.config.tunnel_url) {
                Ok(url) => {
                    if let Some(host) = url.host() {
                        Some(host.serialize())
                    } else {
                        None
                    }
                },
                Err(_) => None
            }
        }
    }
}
