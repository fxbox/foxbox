// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! UPnP listener for Sonos speakers.
//!

extern crate url;

use std::sync::Arc;

use foxbox_taxonomy::manager::*;

use foxbox_core::config_store::ConfigService;
use super::SonosAdapter;
use super::SonosServiceMap;
use foxbox_core::upnp::{UpnpListener, UpnpService};

pub struct SonosUpnpListener {
    manager: Arc<AdapterManager>,
    services: SonosServiceMap,
    config: Arc<ConfigService>,
}

impl SonosUpnpListener {
    pub fn new(manager: &Arc<AdapterManager>,
               services: SonosServiceMap,
               config: &Arc<ConfigService>)
               -> Box<Self> {
        Box::new(SonosUpnpListener {
            manager: manager.clone(),
            services: services,
            config: config.clone(),
        })
    }
}

impl UpnpListener for SonosUpnpListener {
    fn upnp_discover(&self, service: &UpnpService) -> bool {
        macro_rules! try_get {
            ($hash:expr, $key:expr) => (match $hash.get($key) {
                Some(val) => val,
                None => return false
            })
        }

        let device_type = try_get!(service.description, "/root/device/deviceType");
        if device_type != "urn:schemas-upnp-org:device:ZonePlayer:1" {
            return false;
        }

        let model_name = try_get!(service.description, "/root/device/modelName");

        let url = service.msearch.location.clone();

        let mut udn = try_get!(service.description, "/root/device/UDN").clone();
        // The UDN is typically of the for uuid:SOME-UID-HERE, but some devices
        // response with just a UUID. We strip off the uuid: prefix, if it exists
        // and use the resulting UUID as the service id.
        if udn.starts_with("uuid:") {
            udn = String::from(&udn[5..]);
        }

        // TODO: We really need to update the sonos name in the event that
        //       it changed. I'll add this once we start persisting the sonos
        //       information in a database.

        let name = try_get!(service.description, "/root/device/friendlyName").clone();
        let manufacturer = try_get!(service.description, "/root/device/manufacturer");

        SonosAdapter::init_service(&self.manager,
                                   self.services.clone(),
                                   &self.config,
                                   &udn,
                                   &url,
                                   &name,
                                   manufacturer,
                                   model_name)
            .unwrap();
        true
    }
}
