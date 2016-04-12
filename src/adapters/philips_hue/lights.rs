/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Module that implements lights for `PhilipsHueAdapter`
//!
//! This module implements AdapterManager-facing functionality.
//! It registers a service for every light and adds setters and
//! getters according to the light type.

use foxbox_taxonomy::api::Error;
use foxbox_taxonomy::manager::*;
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::{ Type };
use super::*;
use super::hub_api::HubApi;
use std::collections::HashSet;
use std::sync::Arc;
use traits::Controller;

const CUSTOM_PROPERTY_MANUFACTURER: &'static str = "manufacturer";
const CUSTOM_PROPERTY_MODEL: &'static str = "model";
const CUSTOM_PROPERTY_NAME: &'static str = "name";
const CUSTOM_PROPERTY_TYPE: &'static str = "type";

#[derive(Clone)]
pub struct Light {
    api: Arc<HubApi>,
    hub_id: String,
    light_id: String,
    service_id: Id<ServiceId>,
    pub get_available_id: Id<Getter>,
    pub get_power_id: Id<Getter>,
    pub set_power_id: Id<Setter>,
}

impl Light {
    pub fn new(api: Arc<HubApi>, hub_id: &str, light_id: &str)
        -> Self
    {
        Light {
            api: api,
            hub_id: hub_id.to_owned(),
            light_id: light_id.to_owned(),
            service_id: create_light_id(&hub_id, &light_id),
            get_available_id: create_getter_id("available", &hub_id, &light_id),
            get_power_id: create_getter_id("power", &hub_id, &light_id),
            set_power_id: create_setter_id("power", &hub_id, &light_id),
        }
    }
    pub fn start(&self) {
        // Nothing to do, yet
    }
    pub fn stop(&self) {
        // Nothing to do, yet
    }
    pub fn init_service(&mut self, manager: Arc<AdapterManager>,
        services: LightServiceMap) -> Result<(), Error>
    {
        let adapter_id = create_adapter_id();
        let status = self.api.get_light_status(&self.light_id);
        if status.lighttype == "Extended color light" {
            info!("New Philips Hue Extended Color Light service for light {} on bridge {}",
                self.light_id, self.hub_id);
            let mut service = Service::empty(self.service_id.clone(), adapter_id.clone());
            service.properties.insert(CUSTOM_PROPERTY_MANUFACTURER.to_owned(),
                status.manufacturername.to_owned());
            service.properties.insert(CUSTOM_PROPERTY_MODEL.to_owned(),
                status.modelid.to_owned());
            service.properties.insert(CUSTOM_PROPERTY_NAME.to_owned(),
                status.name.to_owned());
            service.properties.insert(CUSTOM_PROPERTY_TYPE.to_owned(),
                "Light/ColorLight".to_owned());
            service.tags.insert(tag_id!("type:Light/ColorLight"));

            try!(manager.add_service(service));

            // The `available` getter yields `On` when the light
            // is plugged in and `Off` when it is not. Availability
            // Has no effect on the API other than that you won't
            // see the light change because it lacks external power.
            try!(manager.add_getter(Channel {
                tags: HashSet::new(),
                adapter: adapter_id.clone(),
                id: self.get_available_id.clone(),
                last_seen: None,
                service: self.service_id.clone(),
                mechanism: Getter {
                    kind: ChannelKind::Extension {
                        vendor: Id::new("foxlink@mozilla.com"),
                        adapter: Id::new("Philips Hue Adapter"),
                        kind: Id::new("available"),
                        typ: Type::OnOff,
                    },
                    updated: None,
                },
            }));

            try!(manager.add_getter(Channel {
                tags: HashSet::new(),
                adapter: adapter_id.clone(),
                id: self.get_power_id.clone(),
                last_seen: None,
                service: self.service_id.clone(),
                mechanism: Getter {
                    kind: ChannelKind::LightOn,
                    updated: None,
                },
            }));

            try!(manager.add_setter(Channel {
                tags: HashSet::new(),
                adapter: adapter_id.clone(),
                id: self.set_power_id.clone(),
                last_seen: None,
                service: self.service_id.clone(),
                mechanism: Setter {
                    kind: ChannelKind::LightOn,
                    updated: None,
                },
            }));

            let mut services_lock = services.lock().unwrap();
            services_lock.getters.insert(self.get_available_id.clone(), self.clone());
            services_lock.getters.insert(self.get_power_id.clone(), self.clone());
            services_lock.setters.insert(self.set_power_id.clone(), self.clone());

        } else {
            warn!("Ignoring unsupported Hue light type {}, ID {} on bridge {}",
                status.lighttype, self.light_id, self.hub_id);
        }
        Ok(())
    }

    pub fn get_available(&self) -> bool {
        let status = self.api.get_light_status(&self.light_id);
        status.state.reachable
    }

    pub fn get_power(&self) -> bool {
        let status = self.api.get_light_status(&self.light_id);
        status.state.on
    }

    pub fn set_power(&self, on: bool) {
        self.api.set_light_power(&self.light_id, on);
    }

}
