/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Assumes Unix
use std::io::prelude::*;
use std::process::{ Child, Command };
use std::io::Result;
use managed_process::ManagedProcess;

pub type TunnelProcess = ManagedProcess;

pub struct Tunnel<T> where T: TunnelConfigTrait {
    config: T,
    pub tunnel_process: Option<TunnelProcess>
}

pub trait TunnelConfigTrait {
    fn new(tunnel_url: String,
           tunnel_secret: String,
           local_http_port: u16,
           local_ws_port: u16,
           remote_name: String) -> Self;
    fn spawn(&self) -> Result<Child>;
}

#[derive(Clone, Debug)]
pub struct TunnelConfig {
    tunnel_url: String,
    tunnel_secret: String,
    local_http_port: u16,
    local_ws_port: u16,
    remote_name: String
}

impl TunnelConfigTrait for TunnelConfig {
    fn new(tunnel_url: String,
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
    fn spawn(&self) -> Result<Child> {
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

impl<T> Tunnel<T> where T: TunnelConfigTrait + Clone + Send + 'static {
    /// Create a new Tunnel object containing the necessary
    /// configuration
    pub fn new(config: T) -> Self {
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
}


#[cfg(test)]
describe! tunnel {

    before_each {
        use std::thread::sleep;
        use std::time::Duration;
        use stubs::tunnel::TunnelConfigStub;

        let config = TunnelConfigStub::stub();
        let mut tunnel = super::Tunnel::new(config.clone());

        tunnel.start().unwrap();
        // XXX Sleep needed otherwise the process is not started in time
        sleep(Duration::from_millis(100));
    }

    after_each {
        tunnel.stop().unwrap();
    }

    describe! start {
        it "should start a new tunnel" {
            let count = *config.spawn_called_count.lock().unwrap();
            assert_eq!(count, 1);
        }

        it "should not start a new tunnel if one is already present" {
            tunnel.start().unwrap();
            sleep(Duration::from_millis(100));

            let count = *config.spawn_called_count.lock().unwrap();
            assert_eq!(count, 1);
        }
    }

    describe! stop {
        it "should stop a tunnel" {
            tunnel.stop().unwrap();
            tunnel.start().unwrap();
            sleep(Duration::from_millis(100));

            let count = *config.spawn_called_count.lock().unwrap();
            assert_eq!(count, 2);
        }
    }
}
