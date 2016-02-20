/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::thread;
use context::{ ContextTrait, SharedContext };
use ws::{ Handler, Sender, Result, Message, Handshake, CloseCode, Error };

use ws::listen;

pub struct WsServer;

pub struct WsHandler {
    pub out: Sender,
    pub context: SharedContext
}

impl WsServer {

    pub fn start(context: SharedContext) {
        thread::Builder::new().name("WsServer".to_owned()).spawn(move || {

            let hostname;
            let port;
            {
                let ctx = context.lock().unwrap();
                hostname = ctx.hostname.clone();
                port = ctx.ws_port;
            }

            listen((&hostname as &str, port), |out| {
                WsHandler {
                    out: out,
                    context: context.clone(),
                }
            }).unwrap();
        }).unwrap();
    }
}

impl Handler for WsHandler {

    fn on_open(&mut self, _: Handshake) -> Result<()> {
        println!("Hello new ws connection");

        self.context.lock().unwrap().add_websocket(self.out.clone());

        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        println!("Message from websocket ({:?}): {}", self.out.token(), msg);

        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => println!("The ws client is done with the connection."),
            CloseCode::Away => println!("The ws client is leaving the site."),
            _ => println!("The ws client encountered an error: {}.", reason),
        }

        self.context.lock().unwrap().remove_websocket(self.out.clone());
    }

    fn on_error(&mut self, err: Error) {
        println!("The ws server encountered an error: {:?}", err);
    }
}
