/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Assumes Unix
use std::io::prelude::*;
use std::fs::File;
use std::process::{ Child, Command };
use std::io::Result;
use managed_process::ManagedProcess;

pub type TunnelProcess = ManagedProcess;

pub struct Tunnel {
    config: TunnelConfig,
    pub tunnel_process: Option<TunnelProcess>
}

#[derive(Clone, Debug)]
pub struct TunnelConfig {
    local_port: u16,
    remote_host: String,
}

impl TunnelConfig {
    pub fn new(port: u16, remote_host: String) -> Self {
        TunnelConfig {
            local_port: port,
            remote_host: remote_host
        }
    }

    /// Describes how to spawn the ngrok process
    pub fn spawn(&self) -> Result<Child> {
        self.write_config_file();
        Command::new("ngrok")
                // Important! By default ngrok has a curses like view
                // which takes over terminal, setting log overrides that
                .arg("-log=stdout")
                .arg("-config")
                .arg("ngrok_config.yaml")
                .arg(self.local_port.to_string())
                .spawn()
    }

    /// Write out the config file that ngrok relies on - this means that this file will always be
    /// around in the right place, it doesn't clean up after itself
    fn write_config_file(&self) -> () {
        let mut file = File::create("ngrok_config.yaml").unwrap();
        let remote_host = self.remote_host.to_owned();

        file.write_fmt(format_args!("server_addr: {}\ntrust_host_root_certs: false", remote_host)).unwrap();
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
    #[allow(dead_code)]
    pub fn stop(&mut self) -> Result<()> {
        match self.tunnel_process.take() {
            None => Ok(()),
            Some(process) => process.shutdown()
        }
    }
}
