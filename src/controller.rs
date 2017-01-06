// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate serde_json;
extern crate mio;

use adapters::AdapterManager;
use foxbox_core::config_store::ConfigService;
use foxbox_core::profile_service::{ProfilePath, ProfileService};
use foxbox_core::traits::Controller;
use foxbox_core::upnp::UpnpManager;
use foxbox_taxonomy::api::{API, Targetted, WatchEvent};
use foxbox_taxonomy::manager::{AdapterManager as TaxoManager, WatchGuard};
use foxbox_taxonomy::selector::ChannelSelector;
use foxbox_taxonomy::util::Exactly;
use foxbox_users::UsersManager;
use http_server::HttpServer;
use mio::{Events, Poll};
use std::collections::hash_map::HashMap;
use std::io;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::vec::IntoIter;
use tls::{CertificateManager, CertificateRecord, SniSslContextProvider, TlsOption};
use transformable_channels::mpsc;
use ws_server::WsServer;
use ws;

#[derive(Clone)]
pub struct FoxBox {
    pub verbose: bool,
    tls_option: TlsOption,
    certificate_manager: CertificateManager,
    hostname: String,
    domain: String,
    http_port: u16,
    ws_port: u16,
    websockets: Arc<Mutex<HashMap<ws::util::Token, ws::Sender>>>,
    pub config: Arc<ConfigService>,
    upnp: Arc<UpnpManager>,
    users_manager: Arc<UsersManager>,
    profile_service: Arc<ProfileService>,
}

impl FoxBox {
    pub fn new(verbose: bool,
               hostname: &str,
               domain: &str,
               http_port: u16,
               ws_port: u16,
               tls_option: TlsOption,
               profile_path: ProfilePath)
               -> Self {

        let profile_service = ProfileService::new(profile_path);
        let config = Arc::new(ConfigService::new(&profile_service.path_for("foxbox.conf")));

        let certificate_directory = PathBuf::from(config.get_or_set_default("foxbox",
                                "certificate_directory",
                                &profile_service.path_for("certs/")));

        FoxBox {
            certificate_manager: CertificateManager::new(certificate_directory,
                                                         domain,
                                                         Box::new(SniSslContextProvider::new())),
            tls_option: tls_option,
            websockets: Arc::new(Mutex::new(HashMap::new())),
            verbose: verbose,
            hostname: hostname.to_owned(),
            domain: domain.to_owned(),
            http_port: http_port,
            ws_port: ws_port,
            config: config,
            upnp: Arc::new(UpnpManager::new()),
            users_manager:
                Arc::new(UsersManager::new(&profile_service.path_for("users_db.sqlite"))),
            profile_service: Arc::new(profile_service),
        }
    }

    #[allow(unused_variables)]
    fn watch_values(&self, taxo_manager: &Arc<TaxoManager>) -> WatchGuard {
        let (tx, rx) = mpsc::channel::<WatchEvent>();
        // We can't use let _ = taxo_manager.watch_values() because that would drop the
        // guard immediately and remove the watcher.
        let watchguard = taxo_manager.watch_values(vec![Targetted {
                                           select: vec![ChannelSelector::new()], // All channels.
                                           payload: Exactly::Always, // All events.
                                       }],
                                  Box::new(tx));

        // This thread will receive the events from the adapters and relay them to websockets.
        let myself = self.clone();
        thread::Builder::new()
            .name("ValueWatcher".to_owned())
            .spawn(move || {
                loop {
                    if let Ok(event) = rx.recv() {
                        match event {
                            WatchEvent::Error { channel, error } => {
                                error!("{} : {}", channel, error)
                            }
                            WatchEvent::ChannelAdded(id) => {
                                info!("Channel Added: {}", id);
                                myself.broadcast_to_websockets(json_value!({ type: "channel/added", id: id }));
                            },
                            WatchEvent::ChannelRemoved(id) => {
                                info!("Channel Removed: {}", id);
                                myself.broadcast_to_websockets(json_value!({ type: "channel/removed", id: id }));
                            }
                            WatchEvent::EnterRange { channel, value, format} => {
                                info!("Entering Range {} : {:?}", channel, value);
                                myself.broadcast_to_websockets(json_value!({ type: "range/enter", channel: channel, value: value }));
                            }
                             WatchEvent::ExitRange { channel, value, format} => {
                                info!("Exiting Range {} : {:?}", channel, value);
                                myself.broadcast_to_websockets(json_value!({ type: "range/exit", channel: channel, value: value }));
                            }
                        }
                    }
                }
            })
            .unwrap();

        watchguard
    }
}

impl Controller for FoxBox {
    fn run(&mut self, shutdown_flag: &AtomicBool) {

        debug!("Starting controller");

        {
            Arc::get_mut(&mut self.upnp).unwrap().start().unwrap();
        }

        // Create the taxonomy based AdapterManager
        let tags_db_path = PathBuf::from(self.profile_service.path_for("taxonomy_tags.sqlite"));
        let taxo_manager = Arc::new(TaxoManager::new(Some(tags_db_path)));

        let guard = self.watch_values(&taxo_manager);

        let mut adapter_manager = AdapterManager::new(self.clone());
        adapter_manager.start(&taxo_manager);

        HttpServer::new(self.clone()).start(&taxo_manager);
        WsServer::start(self.clone());

        let poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(1024);
        loop {
            let _ = poll.poll(&mut events, None);
            if shutdown_flag.load(Ordering::Acquire) {
                break;
            }
        }

        debug!("Stopping controller");
        adapter_manager.stop();
        taxo_manager.stop();
    }

    fn adapter_started(&self, adapter: String) {
        self.broadcast_to_websockets(json_value!({ type: "core/adapter/start", name: adapter }));
    }

    fn adapter_notification(&self, notification: serde_json::value::Value) {
        self.broadcast_to_websockets(json_value!({ type: "core/adapter/notification", message: notification }));
    }

    fn http_as_addrs(&self) -> Result<IntoIter<SocketAddr>, io::Error> {
        ("::", self.http_port).to_socket_addrs()
    }

    fn ws_as_addrs(&self) -> Result<IntoIter<SocketAddr>, io::Error> {
        ("::", self.ws_port).to_socket_addrs()
    }

    fn add_websocket(&mut self, socket: ws::Sender) {
        self.websockets.lock().unwrap().insert(socket.token(), socket);
    }

    fn remove_websocket(&mut self, socket: ws::Sender) {
        self.websockets.lock().unwrap().remove(&socket.token());
    }

    fn broadcast_to_websockets(&self, data: serde_json::value::Value) {
        let serialized = serde_json::to_string(&data).unwrap_or("{}".to_owned());
        debug!("broadcast_to_websockets {}", serialized.clone());
        for socket in self.websockets.lock().unwrap().values() {
            match socket.send(serialized.clone()) {
                Ok(_) => (),
                Err(err) => error!("Error sending to socket: {}", err),
            }
        }
    }

    fn get_config(&self) -> Arc<ConfigService> {
        self.config.clone()
    }

    fn get_profile(&self) -> &ProfileService {
        &self.profile_service
    }

    fn get_upnp_manager(&self) -> Arc<UpnpManager> {
        self.upnp.clone()
    }

    fn get_users_manager(&self) -> Arc<UsersManager> {
        self.users_manager.clone()
    }

    fn get_certificate_manager(&self) -> CertificateManager {
        self.certificate_manager.clone()
    }

    /// Every box should create a self signed certificate for a local name.
    /// The fingerprint of that certificate becomes the box's identifier,
    /// which is used to create the public DNS zone and local
    /// (i.e. local.<fingerprint>.box.knilxof.org) and remote
    /// (i.e. remote.<fingerprint>.box.knilxof.org) origins
    fn get_box_certificate(&self) -> io::Result<CertificateRecord> {
        self.certificate_manager.get_box_certificate()
    }

    fn get_tls_enabled(&self) -> bool {
        self.tls_option == TlsOption::Enabled
    }

    fn get_hostname(&self) -> String {
        self.hostname.clone()
    }

    fn get_domain(&self) -> String {
        self.domain.clone()
    }
}
