/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

mod ip_camera_adapter;

/// An adapter dedicated to the Philips Hue
mod philips_hue;

/// An adapter providing time services.
pub mod clock;

/// An adapter providing WebPush services.
pub mod webpush;

use foxbox_adapters::adapter::AdapterManagerHandle;
use self::ip_camera_adapter::IpCameraAdapter;
use self::philips_hue::PhilipsHueAdapter;
use service::ServiceAdapter;
use traits::Controller;

pub struct AdapterManager<T> {
    controller: T,
    adapters: Vec<Box<ServiceAdapter>>,
}

impl<T: Controller> AdapterManager<T> {
    pub fn new(controller: T) -> Self {
        debug!("Creating Adapter Manager");
        AdapterManager {
            controller: controller,
            adapters: Vec::new(),
        }
    }

    /// Start all the adapters.
    pub fn start<A>(&mut self, adapter_manager: &A)
        where A: AdapterManagerHandle {
        let c = self.controller.clone(); // extracted here to prevent double-borrow of 'self'
        self.start_adapter(Box::new(PhilipsHueAdapter::new(c.clone())));
        self.start_adapter(Box::new(IpCameraAdapter::new(c.clone())));
        clock::Clock::init(adapter_manager).unwrap(); // FIXME: We should have a way to report errors
        webpush::WebPush::init(c, adapter_manager).unwrap();
    }

    fn start_adapter(&mut self, adapter: Box<ServiceAdapter>) {
        adapter.start();
        self.adapters.push(adapter);
    }

    /// Stop all the adapters.
    pub fn stop(&self) {
        for adapter in &self.adapters {
            adapter.stop();
        }
    }
}
