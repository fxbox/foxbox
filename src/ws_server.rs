/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::thread;
use ws::{ Handler, Sender, Result, Message, Handshake, CloseCode, Error };
use ws::listen;
use controller::Controller;

pub struct WsServer;

pub struct WsHandler<T> {
    pub out: Sender,
    pub controller: T
}

impl WsServer {

    pub fn start<T: Controller>(controller: T, hostname: String, port: u16) {
        thread::Builder::new().name("WsServer".to_owned()).spawn(move || {
            listen((&hostname as &str, port), |out| {
                WsHandler {
                    out: out,
                    controller: controller.clone(),
                }
            }).unwrap();
        }).unwrap();
    }
}

impl<T: Controller> Handler for WsHandler<T> {

    fn on_open(&mut self, _: Handshake) -> Result<()> {
        info!("Hello new ws connection");
        self.controller.add_websocket(self.out.clone());
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        info!("Message from websocket ({:?}): {}", self.out.token(), msg);

        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => info!("The ws client is done with the connection."),
            CloseCode::Away => info!("The ws client is leaving the site."),
            _ => error!("The ws client encountered an error: {}.", reason),
        }

        self.controller.remove_websocket(self.out.clone());
    }

    fn on_error(&mut self, err: Error) {
        error!("The ws server encountered an error: {:?}", err);
    }
}
