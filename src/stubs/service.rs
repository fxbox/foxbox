/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
use std::collections::BTreeMap;

use service::{ Service, ServiceProperties };
use iron::{ Request, Response, IronResult };

#[derive(Clone, Copy)]
pub struct ServiceStub;

impl Service for ServiceStub {
    fn get_properties(&self) -> ServiceProperties {
        ServiceProperties {
            id: "1".to_owned(),
            name: "dummy service".to_owned(),
            description: "really nothing to see".to_owned(),
            http_url: "2".to_owned(),
            ws_url: "3".to_owned(),
            custom_properties: BTreeMap::new(),
        }
    }
    fn start(&self) {}
    fn stop(&self) {}
    fn process_request(&self, _: &mut Request) -> IronResult<Response> {
        Ok(Response::with("request processed"))
    }
}
