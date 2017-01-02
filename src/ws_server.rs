// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
extern crate url;

use self::url::Url;
use foxbox_core::traits::Controller;
use openssl::ssl::{Ssl, SslContext};
use std::rc::Rc;
use std::thread;
use ws;
use ws::{Handler, Sender, Result, Message, Handshake, CloseCode, Error};
use ws::listen;

pub struct WsServer;

pub struct WsHandler<T> {
    pub out: Sender,
    pub controller: T,
    ssl: Option<Rc<SslContext>>,
}

impl WsServer {
    pub fn start<T: Controller>(controller: T) {
        let addrs: Vec<_> = controller.ws_as_addrs().unwrap().collect();
        thread::Builder::new()
            .name("WsServer".to_owned())
            .spawn(move || {
                let ssl = {
                    if controller.get_tls_enabled() {
                        let mut context = SslContext::new(SslMethod::Tlsv1)
                            .expect("Creating a SSL context should not fail.");
                        None
                    } else {
                        None
                    }
                };
                listen(addrs[0], |out| {
                        WsHandler {
                            out: out,
                            controller: controller.clone(),
                            ssl: ssl.clone(),
                        }
                    })
                    .unwrap();
            })
            .unwrap();
    }
}

impl<T: Controller> WsHandler<T> {
    fn close_with_error(&mut self, reason: &'static str) -> Result<()> {
        self.out.close_with_reason(ws::CloseCode::Error, reason)
    }
}

impl<T: Controller> Handler for WsHandler<T> {
    fn on_open(&mut self, handshake: Handshake) -> Result<()> {
        info!("Hello new ws connection");

        let resource = &handshake.request.resource()[..];

        // creating a fake url to get the path and query parsed
        let url = match Url::parse(&format!("http://box.fox{}", resource)) {
            Ok(val) => val,
            _ => return self.close_with_error("Invalid path"),
        };

        let auth = url.query_pairs()
            .find(|set| set.0.to_lowercase() == "auth")
            .map(|set| set.1.clone());

        let token = match auth {
            Some(val) => val,
            _ => return self.close_with_error("Missing authorization"),
        };

        if self.controller.get_users_manager().verify_token(&token).is_err() {
            return self.close_with_error("Authorization failed");
        }

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

    fn build_ssl(&mut self) -> ws::Result<Ssl> {
        if self.ssl.is_none() {
            return Err(ws::Error::new(ws::ErrorKind::Internal, "SSL is disabled"));
        }

        Ssl::new(&self.ssl.clone().unwrap()).map_err(ws::Error::from)
    }
}
