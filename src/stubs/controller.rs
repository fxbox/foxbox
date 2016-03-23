/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate rand;

use config_store::ConfigService;
use foxbox_users::UsersManager;
use iron::{Request, Response, IronResult};
use iron::status::Status;
use profile_service::{ ProfilePath, ProfileService };
use std::vec::IntoIter;
use serde_json;
use service::{ Service, ServiceProperties };
use std::io;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tls::CertificateManager;
use traits::Controller;
use upnp::UpnpManager;
use ws;

#[derive(Clone)]
pub struct ControllerStub {
    pub config: Arc<ConfigService>,
    profile_service: Arc<ProfileService>
}

impl ControllerStub {
    pub fn new() -> Self {
        let path = format!("/tmp/{}", rand::random::<i32>());
        let profile_service = ProfileService::new(ProfilePath::Custom(path));
        ControllerStub {
            config: Arc::new(
                ConfigService::new(&profile_service.path_for("foxbox.conf"))
            ),
            profile_service: Arc::new(profile_service)
        }
    }
}

impl Controller for ControllerStub {
    fn run(&mut self, _: &AtomicBool) {}
    fn dispatch_service_request(&self, id: String, request: &mut Request)
        -> IronResult<Response> {
        Ok(Response::with(Status::Ok))
    }
    fn adapter_started(&self, _: String) {}
    fn adapter_notification(&self, _: serde_json::value::Value) {}
    fn add_service(&self, _: Box<Service>) {}
    fn remove_service(&self, _: String) {}
    fn get_service_properties(&self, id: String) -> Option<ServiceProperties> {
        None
    }
    fn services_count(&self) -> usize {
        0
    }
    fn services_as_json(&self) -> Result<String, serde_json::error::Error> {
        Ok(String::from(""))
    }
    fn get_http_root_for_service(&self, service_id: String) -> String {
        String::from("")
    }
    fn get_ws_root_for_service(&self, service_id: String) -> String {
        String::from("")
    }
    fn http_as_addrs(&self) -> Result<IntoIter<SocketAddr>, io::Error> {
        ("localhost", 3000).to_socket_addrs()
    }
    fn add_websocket(&mut self, socket: ws::Sender) {}
    fn remove_websocket(&mut self, socket: ws::Sender) {}
    fn broadcast_to_websockets(&self, data: serde_json::value::Value) {}

    fn get_config(&self) -> &ConfigService {
        &self.config
    }
    fn get_upnp_manager(&self) -> Arc<UpnpManager> {
        Arc::new(UpnpManager::new())
    }
    fn get_users_manager(&self) -> Arc<UsersManager> {
        Arc::new(UsersManager::new(&self.profile_service.path_for("unused")))
    }
    fn get_profile(&self) -> &ProfileService {
        &self.profile_service
    }
    fn get_tls_enabled(&self) -> bool {
        false
    }

    fn get_certificate_manager(&self) -> CertificateManager {
       CertificateManager::new(PathBuf::from(current_dir!()))
    }
}
