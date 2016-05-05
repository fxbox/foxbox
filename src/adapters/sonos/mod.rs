/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! An adapter providing access to IP cameras. Currently only the following IP cameras are
//! supported: DLink DCS-5010L, DLink DCS-5020L and DLink DCS-5025.
//!

extern crate serde_json;

mod upnp_listener;
mod sonos;

use config_store::ConfigService;
use foxbox_taxonomy::api::{Error, InternalError, User};
use foxbox_taxonomy::manager::*;
use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::{ Value, Json, Binary, Type, TypeError};
use traits::Controller;
use transformable_channels::mpsc::*;
use self::upnp_listener::SonosUpnpListener;
use self::sonos::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

const CUSTOM_PROPERTY_MANUFACTURER: &'static str = "manufacturer";
const CUSTOM_PROPERTY_MODEL: &'static str = "model";
const CUSTOM_PROPERTY_NAME: &'static str = "name";
const CUSTOM_PROPERTY_URL: &'static str = "url";
const CUSTOM_PROPERTY_UDN: &'static str = "udn";

static ADAPTER_NAME: &'static str = "Sonos adapter";
static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
static ADAPTER_VERSION: [u32; 4] = [0, 0, 0, 0];

pub type SonosServiceMap = Arc<Mutex<SonosServiceMapInternal>>;

pub struct SonosServiceMapInternal {
    getters: HashMap<Id<Getter>, Arc<Sonos>>,
    setters: HashMap<Id<Setter>, Arc<Sonos>>,
}

pub struct SonosAdapter {
    services: SonosServiceMap,
}

impl SonosAdapter {
    pub fn id() -> Id<AdapterId> {
        Id::new("sonos@link.mozilla.org")
    }

    pub fn init<C>(adapt: &Arc<AdapterManager>, controller: C) -> Result<(), Error>
        where C: Controller
    {
        let services = Arc::new(Mutex::new(SonosServiceMapInternal {
            getters: HashMap::new(),
            setters: HashMap::new(),
        }));
        let sonos_adapter = Arc::new(SonosAdapter {
            services: services.clone(),
        });

        try!(adapt.add_adapter(sonos_adapter));

        // The UPNP listener will add camera service for discovered speakers.
        let upnp = controller.get_upnp_manager();
        let listener = SonosUpnpListener::new(adapt, services, &controller.get_config());
        upnp.add_listener("SonosTaxonomy".to_owned(), listener);

        // Search for Sonos devices.
        upnp.search(Some("urn:schemas-upnp-org:device:ZonePlayer:1".to_owned())).unwrap();
        Ok(())
    }

    pub fn init_service(adapt: &Arc<AdapterManager>,
                        services: SonosServiceMap,
                        config: &Arc<ConfigService>,
                        udn: &str,
                        url: &str,
                        name: &str,
                        manufacturer: &str,
                        model_name: &str) -> Result<(), Error>
    {
        let service_id = create_service_id(udn);

        let adapter_id = Self::id();
        let mut service = Service::empty(service_id.clone(), adapter_id.clone());

        service.properties.insert(CUSTOM_PROPERTY_MANUFACTURER.to_owned(),
                                           manufacturer.to_owned());
        service.properties.insert(CUSTOM_PROPERTY_MODEL.to_owned(), model_name.to_owned());
        service.properties.insert(CUSTOM_PROPERTY_NAME.to_owned(), name.to_owned());
        service.properties.insert(CUSTOM_PROPERTY_URL.to_owned(), url.to_owned());
        service.properties.insert(CUSTOM_PROPERTY_UDN.to_owned(), udn.to_owned());
        service.tags.insert(tag_id!(&format!("name:{}", name)));

        // Since the upnp_discover will be called about once very 3 minutes we want to ignore
        // discoveries if the sonos is already registered.
        if let Err(error) = adapt.add_service(service) {
            if let Error::InternalError(ref internal_error) = error {
                if let InternalError::DuplicateService(_) = *internal_error {
                    debug!("Found {} @ {} UDN {} (ignoring since it already exists)",
                           model_name,
                           url,
                           udn);
                    return Ok(());
                }
            }

            panic!(error);
        }

        info!("Adding Sonos {} Manufacturer: {} Model: {} Name: {} Url: {}",
              udn,
              manufacturer,
              model_name,
              name,
              url);

        /*let getter_image_list_id = create_getter_id("image_list", udn);
        try!(adapt.add_getter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: getter_image_list_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Getter {
                kind: ChannelKind::Extension {
                    vendor: Id::new("foxlink@mozilla.com"),
                    adapter: Id::new("IPCam Adapter"),
                    kind: Id::new("image_list"),
                    typ: Type::Json,
                },
                updated: None,
            },
        }));

        let getter_image_newest_id = create_getter_id("image_newest", udn);
        try!(adapt.add_getter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: getter_image_newest_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Getter {
                kind: ChannelKind::Extension {
                    vendor: Id::new("foxlink@mozilla.com"),
                    adapter: Id::new("IPCam Adapter"),
                    kind: Id::new("latest image"),
                    typ: Type::Binary,
                },
                updated: None,
            },
        }));

        let setter_snapshot_id = create_setter_id("snapshot", udn);
        try!(adapt.add_setter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: setter_snapshot_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Setter {
                kind: ChannelKind::TakeSnapshot,
                updated: None,
            },
        }));

        let getter_username_id = create_getter_id("username", udn);
        try!(adapt.add_getter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: getter_username_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Getter {
                kind: ChannelKind::Username,
                updated: None,
            },
        }));

        let setter_username_id = create_setter_id("username", udn);
        try!(adapt.add_setter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: setter_username_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Setter {
                kind: ChannelKind::Username,
                updated: None,
            },
        }));

        let getter_password_id = create_getter_id("password", udn);
        try!(adapt.add_getter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: getter_password_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Getter {
                kind: ChannelKind::Password,
                updated: None,
            },
        }));

        let setter_password_id = create_setter_id("password", udn);
        try!(adapt.add_setter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: setter_password_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Setter {
                kind: ChannelKind::Password,
                updated: None,
            },
        }));*/

        let mut serv = services.lock().unwrap();
        let sonos = Arc::new(Sonos::new(udn, url, name, config));
        /*serv.getters.insert(getter_image_list_id, camera.clone());
        serv.getters.insert(getter_image_newest_id, camera.clone());
        serv.setters.insert(setter_snapshot_id, camera.clone());
        serv.getters.insert(getter_username_id, camera.clone());
        serv.setters.insert(setter_username_id, camera.clone());
        serv.getters.insert(getter_password_id, camera.clone());
        serv.setters.insert(setter_password_id, camera.clone());*/

        Ok(())
    }
}

impl Adapter for SonosAdapter {
    fn id(&self) -> Id<AdapterId> {
        Self::id()
    }

    fn name(&self) -> &str {
        ADAPTER_NAME
    }

    fn vendor(&self) -> &str {
        ADAPTER_VENDOR
    }

    fn version(&self) -> &[u32; 4] {
        &ADAPTER_VERSION
    }

    fn fetch_values(&self,
                    mut set: Vec<Id<Getter>>,
                    _: User)
                    -> ResultMap<Id<Getter>, Option<Value>, Error> {
        set.drain(..).map(|id| {
            let device = match self.services.lock().unwrap().getters.get(&id) {
                Some(device) => device.clone(),
                None => return (id.clone(), Err(Error::InternalError(InternalError::NoSuchGetter(id))))
            };

            /*if id == camera.get_username_id {
                let rsp = camera.get_username();
                return (id, Ok(Some(Value::String(Arc::new(rsp)))));
            }

            if id == camera.get_password_id {
                let rsp = camera.get_password();
                return (id, Ok(Some(Value::String(Arc::new(rsp)))));
            }

            if id == camera.image_list_id {
                let rsp = camera.get_image_list();
                return (id, Ok(Some(Value::Json(Arc::new(Json(serde_json::to_value(&rsp)))))));
            }

            if id == camera.image_newest_id {
                return match camera.get_newest_image() {
                    Ok(rsp) => (id, Ok(Some(Value::Binary(Binary {
                        data: Arc::new(rsp),
                        mimetype: Id::new("image/jpeg")
                    })))),
                    Err(err) => (id, Err(err))
                };
            }*/

            (id.clone(), Err(Error::InternalError(InternalError::NoSuchGetter(id))))
        }).collect()
    }

    fn send_values(&self, mut values: HashMap<Id<Setter>, Value>, _: User) -> ResultMap<Id<Setter>, (), Error> {
        values.drain().map(|(id, value)| {
            let device = match self.services.lock().unwrap().setters.get(&id) {
                Some(device) => device.clone(),
                None => { return (id, Err(Error::InternalError(InternalError::InvalidInitialService))); }
            };

            /*if id == camera.set_username_id {
                if let Value::String(ref username) = value {
                    camera.set_username(username);
                    return (id, Ok(()));
                }
                return (id, Err(Error::TypeError(TypeError {
                                got:value.get_type(),
                                expected: Type::String
                            })))
            }

            if id == camera.set_password_id {
                if let Value::String(ref password) = value {
                    camera.set_password(password);
                    return (id, Ok(()));
                }
                return (id, Err(Error::TypeError(TypeError {
                                got:value.get_type(),
                                expected: Type::String
                            })))
            }

            if id == camera.snapshot_id {
                return match camera.take_snapshot() {
                    Ok(_) => (id, Ok(())),
                    Err(err) => (id, Err(err))
                };
            }*/

            (id.clone(), Err(Error::InternalError(InternalError::NoSuchSetter(id))))
        }).collect()
    }

    fn register_watch(&self, mut watch: Vec<WatchTarget>) -> WatchResult
    {
        watch.drain(..).map(|(id, _, _)| {
            (id.clone(), Err(Error::GetterDoesNotSupportWatching(id)))
        }).collect()
    }
}
