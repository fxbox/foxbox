/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

mod philips_hue;

use controller::Controller;
use service::ServiceAdapter;

pub struct AdapterManager<T> {
    controller: T
}

impl<T: Controller> AdapterManager<T> {
    pub fn new(controller: T) -> Self {
        debug!("Creating Adapter Manager");
        AdapterManager { controller: controller }
    }

    pub fn start(&self) {
        // Start all the adapters.
        philips_hue::PhilipsHueAdapter::new(self.controller.clone()).start();
	}
}
