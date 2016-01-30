/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate mio;

use context::{ Context, SharedContext };
use dummy_adapter::DummyAdapter;
use events::*;
use http_server::HttpServer;
use mio::{ EventLoop, EventSet, Token };
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
    pub fn new(sender: EventSender, verbose: bool) -> Controler {
        Controler {
            sender: sender,
            context: Context::shared(verbose)
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

    fn ready(&mut self,
             event_loop: &mut EventLoop<Controler>,
             token: Token,
             events: EventSet) {
        println!("Receiving a fd event!");
    }

    fn notify(&mut self,
              event_loop: &mut EventLoop<Controler>,
              data: EventData) {
        println!("Receiving a notification! {}", data.description());

        let mut context = self.context.lock().unwrap();
        match data {
            EventData::ServiceStart { id } => {
                println!("ServiceStart {} We now have {} services.", id, context.services.len());
            }
            EventData::ServiceStop { id } => {
                context.services.remove(&id);
                println!("ServiceStop {} We now have {} services.", id, context.services.len());
            }
            _ => { }
        }
    }
}
