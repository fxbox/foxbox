/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate mio;

use context::SharedContext;
use dummy_adapter::DummyAdapter;
use events::*;
use http_server::HttpServer;
use mio::EventLoop;
use service::{ Service, ServiceAdapter };

pub struct Controler {
    sender: EventSender,
    context: SharedContext
}

impl Controler {
    /// Construct a new `Controler`.
    ///
    /// ```
    /// # use service_manager::Controler;
    /// let controler = Controller::new();
    /// ```
    pub fn new(sender: EventSender, context: SharedContext) -> Controler {
        Controler {
            sender: sender,
            context: context
        }
    }

    pub fn start(&self) {
        println!("Starting controller");

        // Start the http server.
        let http_server = HttpServer::new(self.context.clone());
        http_server.start();

        // Start the dummy adapter.
        let dummy_adapter = DummyAdapter::new(self.sender.clone(), self.context.clone());
        dummy_adapter.start();
    }
}

impl mio::Handler for Controler {
    type Timeout = ();
    type Message = EventData;

    fn notify(&mut self,
              _: &mut EventLoop<Controler>,
              data: EventData) {
        println!("Receiving a notification! {}", data.description());

        let mut context = self.context.lock().unwrap();
        match data {
            EventData::ServiceStart { id } => {
                println!("ServiceStart {} We now have {} services.", id, context.services_count());
            }
            EventData::ServiceStop { id } => {
                context.remove_service(id.clone());
                println!("ServiceStop {} We now have {} services.", id, context.services_count());
            }
            _ => { }
        }
    }
}
