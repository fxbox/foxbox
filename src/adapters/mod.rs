/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

/// An adapter dedicated to the Philips Hue
mod philips_hue;

/// An adapter providing time services.
pub mod clock;

/// An adapter providing `WebPush` services.
pub mod webpush;

mod ip_camera;

/// An adapter providing access to `Thinkerbell`.
mod thinkerbell;

use foxbox_taxonomy::manager::AdapterManager as TaxoManager;

use self::philips_hue::PhilipsHueAdapter;
use self::thinkerbell::ThinkerbellAdapter;
use service::ServiceAdapter;
use traits::Controller;

use std::sync::Arc;

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
    pub fn start(&mut self, manager: &Arc<TaxoManager>) {
        let c = self.controller.clone(); // extracted here to prevent double-borrow of 'self'
        self.start_adapter(Box::new(PhilipsHueAdapter::new(c.clone())));
        clock::Clock::init(manager).unwrap(); // FIXME: We should have a way to report errors
        webpush::WebPush::init(c, manager).unwrap();
        ip_camera::IPCameraAdapter::init(manager, self.controller.clone()).unwrap();
        ThinkerbellAdapter::init(manager).unwrap(); // FIXME: no unwrap!
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
