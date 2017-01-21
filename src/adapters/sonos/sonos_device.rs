// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate hyper;
extern crate time;
extern crate url;

use foxbox_core::config_store::ConfigService;
use foxbox_taxonomy::api::{Error, InternalError};
use foxbox_taxonomy::channel::*;
use foxbox_taxonomy::services::*;
use rustc_serialize::base64::{FromBase64, ToBase64, STANDARD};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::io::{BufWriter, ErrorKind};
use std::io::prelude::*;
use std::path::Path;
use std::sync::Arc;

pub fn create_service_id(service_id: &str) -> Id<ServiceId> {
    Id::new(&format!("service:{}@link.mozilla.org", service_id))
}

pub fn create_channel_id(operation: &str, service_id: &str) -> Id<Channel> {
    Id::new(&format!("channel:{}.{}@link.mozilla.org", operation, service_id))
}

#[derive(Clone)]
pub struct Sonos {
    pub udn: String,
    url: String,
    config: Arc<ConfigService>,
    upnp_name: String, /* pub image_list_id: Id<Getter>,
                        * pub image_newest_id: Id<Getter>,
                        * pub snapshot_id: Id<Setter>,
                        * pub get_username_id: Id<Getter>,
                        * pub set_username_id: Id<Setter>,
                        * pub get_password_id: Id<Getter>,
                        * pub set_password_id: Id<Setter>, */
}

impl Sonos {
    pub fn new(udn: &str, url: &str, upnp_name: &str, config: &Arc<ConfigService>) -> Self {
        Sonos {
            udn: udn.to_owned(),
            url: url.to_owned(),
            config: config.clone(),
            upnp_name: upnp_name.to_owned(), /* image_list_id: create_getter_id("image_list", &udn),
                                              * image_newest_id: create_getter_id("image_newest", &udn),
                                              * snapshot_id: create_setter_id("snapshot", &udn),
                                              * get_username_id: create_getter_id("username", &udn),
                                              * set_username_id: create_setter_id("username", &udn),
                                              * get_password_id: create_getter_id("password", &udn),
                                              * set_password_id: create_setter_id("password", &udn), */
        }
    }
}
