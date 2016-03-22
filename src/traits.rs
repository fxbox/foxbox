/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use config_store::ConfigService;
use core::marker::Reflect;
use foxbox_users::UsersManager;
use iron::{ IronResult, Response, Request };
use profile_service::ProfileService;
use serde_json;
use service::{ Service, ServiceProperties };
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::vec::IntoIter;
use tls::CertificateManager;
use upnp::UpnpManager;
use ws;

pub trait Controller : Send + Sync + Clone + Reflect + 'static {
    fn run(&mut self, shutdown_flag: &AtomicBool);
    fn dispatch_service_request(&self, id: String, request: &mut Request) -> IronResult<Response>;
    fn adapter_started(&self, adapter: String);
    fn adapter_notification(&self, notification: serde_json::value::Value);
    fn add_service(&self, service: Box<Service>);
    fn remove_service(&self, id: String);
    fn get_service_properties(&self, id: String) -> Option<ServiceProperties>;
    fn services_count(&self) -> usize;
    fn services_as_json(&self) -> Result<String, serde_json::error::Error>;
    fn get_http_root_for_service(&self, service_id: String) -> String;
    fn get_ws_root_for_service(&self, service_id: String) -> String;
    fn http_as_addrs(&self) -> Result<IntoIter<SocketAddr>, io::Error>;

    fn get_tls_enabled(&self) -> bool;
    fn get_certificate_manager(&self) -> CertificateManager;

    fn add_websocket(&mut self, socket: ws::Sender);
    fn remove_websocket(&mut self, socket: ws::Sender);
    fn broadcast_to_websockets(&self, data: serde_json::value::Value);

    fn get_config(&self) -> &ConfigService;
    fn get_upnp_manager(&self) -> Arc<UpnpManager>;
    fn get_users_manager(&self) -> Arc<UsersManager>;
    fn get_profile(&self) -> &ProfileService;
}
