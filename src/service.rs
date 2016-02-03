/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde;

use iron::{Request, Response, IronResult};
use self::serde::ser::{ Serialize, Serializer };

pub type ServiceID = String;

#[derive(Clone, Serialize)]
pub struct ServiceProperties {
    pub id: ServiceID,
    pub name: String,
    pub description: String,
    pub http_url: String,
    pub ws_url: String
}

/// A `Service` represents an individual device.
pub trait Service : Send {
    fn get_properties(&self) -> ServiceProperties;
    fn start(&self);
    fn stop(&self);
    fn process_request(&self, req: &Request) -> IronResult<Response>;
}

impl Serialize for Service {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer {
        let props = self.get_properties();
        props.serialize(serializer)
    }
}

/// A `ServiceAdapter` implements functionality to discover and register a specific type of IoT
/// device protocol. A `ServiceAdapter` can instantiate multiple `Service`s, each representing
/// an individual device.
///
/// For instance, a `LightBulbAdapter` might instantiate a `LightBulbService` for each light bulb
/// it discovers, registering each with the global `Context`.
pub trait ServiceAdapter {
    fn get_name(&self) -> String;
    fn start(&self);
    fn stop(&self);
}
