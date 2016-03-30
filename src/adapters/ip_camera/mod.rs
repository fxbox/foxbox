/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! An adapter providing access to IP cameras. Currently only the following IP cameras are
//! supported: DLink DCS-5010L, DLink DCS-5020L and DLink DCS-5025.
//!

extern crate serde_json;

mod api;
mod upnp_listener;

use foxbox_taxonomy::api::{Error, InternalError};
use foxbox_taxonomy::manager::*;
use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::services::*;
use foxbox_taxonomy::values::{Range, Value, Json, Binary, Type};
use traits::Controller;
use transformable_channels::mpsc::*;
use self::api::IpCamera;
use self::upnp_listener::IpCameraUpnpListener;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

const CUSTOM_PROPERTY_MANUFACTURER: &'static str = "manufacturer";
const CUSTOM_PROPERTY_MODEL: &'static str = "model";
const CUSTOM_PROPERTY_NAME: &'static str = "name";
const CUSTOM_PROPERTY_URL: &'static str = "url";
const CUSTOM_PROPERTY_UDN: &'static str = "udn";

static ADAPTER_NAME: &'static str = "IP Camera adapter";
static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
static ADAPTER_VERSION: [u32; 4] = [0, 0, 0, 0];
static SNAPSHOT_DIR: &'static str = "snapshots";

pub type IpCameraServiceMap = Arc<Mutex<IpCameraServiceMapInternal>>;

pub struct IpCameraServiceMapInternal {
    getters: HashMap<Id<Getter>, IpCamera>,
    setters: HashMap<Id<Setter>, IpCamera>,
    snapshot_root: String,
}

pub struct IPCameraAdapter {
    services: IpCameraServiceMap,
}

impl IPCameraAdapter {
    pub fn id() -> Id<AdapterId> {
        Id::new("ip-camera@link.mozilla.org")
    }

    fn create_service_id(service_id: &str) -> Id<ServiceId> {
        Id::new(&format!("service:{}@link.mozilla.org", service_id))
    }

    pub fn create_setter_id(operation: &str, service_id: &str) -> Id<Setter> {
        Self::create_io_mechanism_id("setter", operation, service_id)
    }

    pub fn create_getter_id(operation: &str, service_id: &str) -> Id<Getter> {
        Self::create_io_mechanism_id("getter", operation, service_id)
    }

    fn create_io_mechanism_id<IO>(prefix: &str, operation: &str, service_id: &str) -> Id<IO>
        where IO: IOMechanism
    {
        Id::new(&format!("{}:{}.{}@link.mozilla.org", prefix, operation, service_id))
    }

    pub fn init<C>(adapt: &Arc<AdapterManager>, controller: C) -> Result<(), Error>
        where C: Controller
    {
        let services = Arc::new(Mutex::new(IpCameraServiceMapInternal {
            getters: HashMap::new(),
            setters: HashMap::new(),
            snapshot_root: controller.get_profile().path_for(SNAPSHOT_DIR),
        }));
        let ip_camera_adapter = Arc::new(IPCameraAdapter {
            services: services.clone()
        });

        try!(adapt.add_adapter(ip_camera_adapter));

        // The UPNP listener will add camera service for discovered cameras
        let upnp = controller.get_upnp_manager();
        let listener = IpCameraUpnpListener::new(adapt, services);
        upnp.add_listener("IpCameraTaxonomy".to_owned(), listener);

        // The UPNP service searches for ssdp:all which the D-Link cameras
        // don't seem to respond to. So we search for this instead, which
        // they do respond to.
        upnp.search(Some("urn:cellvision:service:Null:1".to_owned())).unwrap();
        Ok(())
    }

    pub fn init_service(adapt: &Arc<AdapterManager>, services: IpCameraServiceMap,
        udn: &str, url: &str, name: &str, manufacturer: &str, model_name: &str) -> Result<(), Error>
    {
        let service_id = Self::create_service_id(udn);

        let adapter_id = Self::id();
        let mut service_metadata = Service::empty(service_id.clone(), adapter_id.clone());

        service_metadata.properties.insert(CUSTOM_PROPERTY_MANUFACTURER.to_owned(),
                                           manufacturer.to_owned());
        service_metadata.properties.insert(CUSTOM_PROPERTY_MODEL.to_owned(), model_name.to_owned());
        service_metadata.properties.insert(CUSTOM_PROPERTY_NAME.to_owned(), name.to_owned());
        service_metadata.properties.insert(CUSTOM_PROPERTY_URL.to_owned(), url.to_owned());
        service_metadata.properties.insert(CUSTOM_PROPERTY_UDN.to_owned(), udn.to_owned());

        // Since the upnp_discover will be called about once very 3 minutes we want to ignore
        // discoveries if the camera is already registered.
        if let Err(error) = adapt.add_service(service_metadata) {
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

        debug!("Adding IpCamera {} Manufacturer: {} Model: {} Name: {}",
               udn,
               manufacturer,
               model_name,
               name);

        let getter_image_list_id = Self::create_getter_id("image_list", udn);
        try!(adapt.add_getter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: getter_image_list_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Getter {
                kind: ChannelKind::Extension {
                    vendor: Id::new("DLink"),
                    adapter: Id::new("Adapter"),
                    kind: Id::new("image_list"),
                    typ: Type::Json,
                },
                updated: None,
            },
        }));

        let getter_image_newest_id = Self::create_getter_id("image_newest", udn);
        try!(adapt.add_getter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: getter_image_newest_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Getter {
                kind: ChannelKind::Extension {
                    vendor: Id::new("DLink"),
                    adapter: Id::new("Adapter"),
                    kind: Id::new("image_newest"),
                    typ: Type::Binary,
                },
                updated: None,
            },
        }));

        let setter_snapshot_id = Self::create_setter_id("snapshot", udn);
        try!(adapt.add_setter(Channel {
            tags: HashSet::new(),
            adapter: adapter_id.clone(),
            id: setter_snapshot_id.clone(),
            last_seen: None,
            service: service_id.clone(),
            mechanism: Setter {
                // FIXME: doesn't actually take any params
                kind: ChannelKind::Extension {
                    vendor: Id::new("DLink"),
                    adapter: Id::new("Adapter"),
                    kind: Id::new("snapshot"),
                    typ: Type::Json,
                },
                updated: None,
            },
        }));

        let mut serv = services.lock().unwrap();
        let camera = try!(IpCamera::new(udn, url, &serv.snapshot_root));
        serv.getters.insert(getter_image_list_id, camera.clone());
        serv.getters.insert(getter_image_newest_id, camera.clone());
        serv.setters.insert(setter_snapshot_id, camera);

        Ok(())
    }
}

impl Adapter for IPCameraAdapter {
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
                    mut set: Vec<Id<Getter>>)
                    -> ResultMap<Id<Getter>, Option<Value>, Error> {
        set.drain(..).map(|id| {
            let camera = match self.services.lock().unwrap().getters.get(&id) {
                Some(camera) => camera.clone(),
                None => { return (id, Err(Error::InternalError(InternalError::InvalidInitialService))); }
            };

            if id == Self::create_getter_id("image_list", &camera.udn) {
                let rsp = camera.get_image_list();
                return (id, Ok(Some(Value::Json(Arc::new(Json(serde_json::to_value(&rsp)))))));
            }

            if id == Self::create_getter_id("image_newest", &camera.udn) {
                return match camera.get_newest_image() {
                    Ok(rsp) => (id, Ok(Some(Value::Binary(Binary {
                        data: Arc::new(rsp),
                        mimetype: Id::new("image/jpeg")
                    })))),
                    Err(err) => (id, Err(err))
                };
            }

            (id.clone(), Err(Error::InternalError(InternalError::NoSuchGetter(id))))
        }).collect()
    }

    fn send_values(&self, mut values: HashMap<Id<Setter>, Value>) -> ResultMap<Id<Setter>, (), Error> {
        values.drain().map(|(id, _)| {
            let camera = match self.services.lock().unwrap().setters.get(&id) {
                Some(camera) => camera.clone(),
                None => { return (id, Err(Error::InternalError(InternalError::InvalidInitialService))); }
            };

            if id == Self::create_setter_id("snapshot", &camera.udn) {
                return match camera.take_snapshot() {
                    Ok(_) => (id, Ok(())),
                    Err(err) => (id, Err(err))
                };
            }

            (id.clone(), Err(Error::InternalError(InternalError::NoSuchSetter(id))))
        }).collect()
    }

    fn register_watch(&self,
                      mut watch: Vec<(Id<Getter>, Option<Range>)>,
                      _: Box<ExtSender<WatchEvent>>)
                      -> ResultMap<Id<Getter>, Box<AdapterWatchGuard>, Error> {
        watch.drain(..).map(|(id, _)| {
            (id.clone(), Err(Error::GetterDoesNotSupportWatching(id)))
        }).collect()
    }
}
