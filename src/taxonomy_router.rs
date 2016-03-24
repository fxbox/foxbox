/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

use foxbox_taxonomy::manager::WatchGuard;
use foxbox_taxonomy::api::{ API, Error, TargetMap, ResultMap };
use foxbox_taxonomy::values::{ Binary, Value };
use foxbox_taxonomy::selector::*;
use foxbox_taxonomy::services::*;

use foxbox_users::AuthEndpoint;

use iron::{ Handler, IronResult, Request, Response };
use iron::headers::ContentType;
use iron::method::Method;
use iron::prelude::Chain;
use iron::request::Body;
use iron::status::Status;

use std::io::{ Error as IOError, Read };
use std::sync::{ Arc, Mutex };
use traits::Controller;

/// This is a specialized Router for the taxonomy API.
/// It handles all the calls under the api/v1/ url space.
pub struct TaxonomyRouter<A> {
    api: Mutex<A>
}

type GetterResultMap = ResultMap<Id<Getter>, Option<Value>, Error>;

impl<A> TaxonomyRouter<A>
    where A: API<WatchGuard=WatchGuard> + 'static {
    pub fn new(adapter_api: A) -> Self {
        TaxonomyRouter {
            // This locks the full api access when dealing with a http request,
            // which basically kills http concurrency.
            // TODO: make it concurrent again.
            api: Mutex::new(adapter_api)
        }
    }

    fn build_binary_response(&self, map: &GetterResultMap) -> IronResult<Response> {
        use core::ops::Deref;
        use hyper::mime::Mime;

        let mut payload: Binary =
            Binary { data: Arc::new(vec![]),
                     mimetype: Id::new("application/octet-stream")
                   };

        for map_value in map.values() {
            match *map_value {
                Err(_) => {},
                Ok(ref opt_res) => {
                    match *opt_res {
                        None => {},
                        Some(ref value) => {
                            if let Value::Binary(ref data) = *value {
                                payload = Binary {
                                    mimetype: (*data).mimetype.clone(),
                                    data: (*data).data.clone()
                                }
                            }
                        }
                    }
                }
            }
        };

        let mime : Mime = format!("{}", payload.mimetype).parse().unwrap();
        // TODO: stop copying the array here.
        let data = payload.data.deref().clone();

        let mut response = Response::with(data);
        response.status = Some(Status::Ok);
        response.headers.set(ContentType(mime));
        Ok(response)
    }

    fn build_response<S: ToJSON>(&self, obj: S) -> IronResult<Response> {
        let json = obj.to_json();
        let serialized = itry!(serde_json::to_string(&json));
        let mut response = Response::with(serialized);
        response.status = Some(Status::Ok);
        response.headers.set(ContentType::json());
        Ok(response)
    }

    fn build_parse_error(&self, obj: &ParseError) -> IronResult<Response> {
        let mut response = Response::with(format!("{}", obj));
        response.status = Some(Status::BadRequest);
        response.headers.set(ContentType::plaintext()); // FIXME: Should be JSON
        Ok(response)
    }

    fn read_body_to_string<'a, 'b : 'a>(body: &mut Body<'a, 'b>) -> Result<String, IOError> {
        let mut s = String::new();
        try!(body.read_to_string(&mut s));
        Ok(s)
    }

    // Checks if a getter result map is a binary payload.
    fn is_binary(&self, map: &GetterResultMap) -> bool {
        // For now, consider as binary a result map with a single element that
        // holds a binary value.
        if map.len() != 1 {
            return false;
        }

        for map_value in map.values() {
            match *map_value {
                Err(_) => { return false; },
                Ok(ref opt_res) => {
                    match *opt_res {
                        None => { return false; },
                        Some(ref value) => {
                            return match *value {
                                Value::Binary(_) => true,
                                _ => false
                            };
                        }
                    }
                }
            }
        }

        false
    }
}

impl<A> Handler for TaxonomyRouter<A>
    where A: API<WatchGuard=WatchGuard> + 'static {

    #[allow(cyclomatic_complexity)]
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        // The CORS middleware will take care of adding the CORS headers
        // if they are allowed.
        if req.method == Method::Options {
            return Ok(Response::with(Status::Ok));
        }

        // We are handled urls relative to the mounter set up in http_server.rs
        // That means that for a full url like http://localhost/api/v1/services
        // the req.url.path will only contain ["services"]
        let path = req.url.path.clone();

        /// Generates the code for a generic HTTP call, where we use an empty
        /// taxonomy selector for GET requests, and a decoded json body for POST ones.
        /// $call is the method we'll call on the api, like get_services.
        /// $sel  is the selector type, like ServiceSelector
        /// $path is a vector describing the url path, like ["service", "tags"]
        macro_rules! get_post_api {
            ($call:ident, $sel:ident, $path:expr) => (
            if path == $path {
                return {
                    let api = self.api.lock().unwrap();

                    if req.method != Method::Get && req.method != Method::Post {
                        // Wrong method, return a 405 error.
                        return Ok(Response::with((Status::MethodNotAllowed,
                                                  format!("Bad method: {}", req.method))));
                    }

                    if req.method == Method::Get {
                        // On a GET, just send the full taxonomy content for
                        // this kind of selector.
                        let result = api.$call(vec![$sel::new()]);
                        self.build_response(&result)
                    } else {
                        let source = itry!(Self::read_body_to_string(&mut req.body));
                        match Vec::<$sel>::from_str(&source as &str) {
                            Ok(arg) => self.build_response(&api.$call(arg)),
                            Err(err) => self.build_parse_error(&err)
                        }
                    }
                }
            })
        }

        // Generates the code to process a given HTTP call with a json body.
        macro_rules! payload_api {
            ($call:ident, $param:ty, $path:expr, $method:expr) => (
                if path == $path && req.method == $method {
                    type Selectors = $param;
                    return {
                        let api = self.api.lock().unwrap();
                        let source = itry!(Self::read_body_to_string(&mut req.body));
                        match Selectors::from_str(&source as &str) {
                            Ok(arg) => self.build_response(&api.$call(arg)),
                            Err(err) => self.build_parse_error(&err)
                        }
                    }
                }
            )
        }

        // Generates the code to process a given HTTP call with a json body.
        // Checks if we can send back a binary response.
        // TODO: share code with the payload_api
        macro_rules! payload_api_binary {
            ($call:ident, $param:ty, $path:expr, $method:expr) => (
                if path == $path && req.method == $method {
                    type Selectors = $param;
                    return {
                        let api = self.api.lock().unwrap();
                        let source = itry!(Self::read_body_to_string(&mut req.body));
                        match Selectors::from_str(&source as &str) {
                            Ok(arg) => {
                                let res = api.$call(arg);
                                if self.is_binary(&res) {
                                    self.build_binary_response(&res)
                                } else {
                                    self.build_response(&res)
                                }
                            }
                            Err(err) => self.build_parse_error(&err)
                        }
                    }
                }
            )
        }

        // Generates the code to process a given HTTP call with a json body.
        // This version takes 2 parameters for the internal call.
        macro_rules! payload_api2 {
            ($call:ident, $name1:ident => $param1:ty, $name2:ident => $param2:ty, $path:expr, $method:expr) => (
                if path == $path && req.method == $method {
                    type Param1 = $param1;
                    type Param2 = $param2;
                    return {
                        let api = self.api.lock().unwrap();

                        let source = itry!(Self::read_body_to_string(&mut req.body));
                        let mut json = match serde_json::de::from_str(&source as &str) {
                            Err(err) => return self.build_parse_error(&ParseError::json(err)),
                            Ok(args) => args
                        };
                        let arg_1 = match Param1::take(Path::new(), &mut json, stringify!($name1)) {
                            Err(err) => return self.build_parse_error(&err),
                            Ok(val) => val
                        };
                        let arg_2 = match Param2::take(Path::new(), &mut json, stringify!($name2)) {
                            Err(err) => return self.build_parse_error(&err),
                            Ok(val) => val
                        };
                        self.build_response(&api.$call(arg_1, arg_2))
                    }
                }
            )
        }

        // Keep these urls in sync with the AuthEndpoint(s) in the create() method.

        // Selectors queries.
        get_post_api!(get_services, ServiceSelector, ["services"]);
        get_post_api!(get_getter_channels, GetterSelector, ["channels", "getters"]);
        get_post_api!(get_setter_channels, SetterSelector, ["channels", "setters"]);

        // Fetching and getting values.
        // We can't use a GET http method here because the Fetch() DOM api
        // doesn't allow bodies with GET and HEAD requests.
        payload_api_binary!(fetch_values, Vec<GetterSelector>, ["channels", "get"], Method::Put);
        payload_api!(send_values, TargetMap<SetterSelector, Value>, ["channels", "set"], Method::Put);

        // Adding tags.
        payload_api2!(add_service_tags,
                      services => Vec<ServiceSelector>,
                      tags => Vec<Id<TagId>>,
                      ["services", "tags"], Method::Post);
        payload_api2!(add_getter_tags,
                      getters => Vec<GetterSelector>,
                      tags => Vec<Id<TagId>>,
                      ["channels", "getter", "tags"], Method::Post);
        payload_api2!(add_setter_tags,
                      setters => Vec<SetterSelector>,
                      tags => Vec<Id<TagId>>,
                      ["channels", "getter", "tags"], Method::Post);

        // Removing tags.
        payload_api2!(remove_service_tags,
                      services => Vec<ServiceSelector>,
                      tags => Vec<Id<TagId>>,
                      ["services", "tags"], Method::Delete);
        payload_api2!(remove_getter_tags,
                      getters => Vec<GetterSelector>,
                      tags => Vec<Id<TagId>>,
                      ["channels", "getter", "tags"], Method::Delete);
        payload_api2!(remove_setter_tags,
                      setters => Vec<SetterSelector>,
                      tags => Vec<Id<TagId>>,
                      ["channels", "getter", "tags"], Method::Delete);

        // Fallthrough, returning a 404.
        Ok(Response::with((Status::NotFound,
                           format!("Unknown url: {}", req.url))))
    }
}

pub fn create<T, A>(controller: T, adapter_api: A) -> Chain
    where A: API<WatchGuard=WatchGuard> + 'static,
          T: Controller {
    let router = TaxonomyRouter::new(adapter_api);

    let auth_endpoints = if cfg!(feature = "authentication") && !cfg!(test) {
        // Keep this list in sync with all the (url path, http method) from
        // the handle() method and with the CORS chain in http_server.rs
        vec![
            AuthEndpoint(vec![Method::Get, Method::Post], "services".to_owned()),
            AuthEndpoint(vec![Method::Post, Method::Delete], "services/tags".to_owned()),
            AuthEndpoint(vec![Method::Get, Method::Post], "channels/getters".to_owned()),
            AuthEndpoint(vec![Method::Get, Method::Post], "channels/setters".to_owned()),
            AuthEndpoint(vec![Method::Get], "channels/get".to_owned()),
            AuthEndpoint(vec![Method::Put], "channels/set".to_owned()),
            AuthEndpoint(vec![Method::Post, Method::Delete], "channel/getters/tags".to_owned()),
            AuthEndpoint(vec![Method::Post, Method::Delete], "channel/setters/tags".to_owned())
        ]
    } else {
        vec![]
    };

    let mut chain = Chain::new(router);
    chain.around(controller.get_users_manager().get_middleware(auth_endpoints));

    chain
}

#[cfg(test)]
describe! taxonomy_router {
    before_each {
        extern crate serde_json;

        use adapters::clock;
        use foxbox_taxonomy::manager::AdapterManager;
        use iron::Headers;
        use iron_test::{ request, response };
        use mount::Mount;
        use stubs::controller::ControllerStub;

        let taxo_manager = AdapterManager::new();
        clock::Clock::init(&taxo_manager).unwrap();

        let mut mount = Mount::new();
        mount.mount("/api/v1", create(ControllerStub::new(), taxo_manager));
    }

    it "should return the list of services from a GET request" {
        let response = request::get("http://localhost:3000/api/v1/services",
                                    Headers::new(),
                                    &mount).unwrap();
        let body = response::extract_body_to_string(response);
        let s = r#"[{"adapter":"clock@link.mozilla.org","getters":{"getter:timeofday.clock@link.mozilla.org":{"adapter":"clock@link.mozilla.org","id":"getter:timeofday.clock@link.mozilla.org","kind":"CurrentTimeOfDay","mechanism":"getter","service":"service:clock@link.mozilla.org","tags":[]},"getter:timestamp.clock@link.mozilla.org":{"adapter":"clock@link.mozilla.org","id":"getter:timestamp.clock@link.mozilla.org","kind":"CurrentTime","mechanism":"getter","service":"service:clock@link.mozilla.org","tags":[]}},"id":"service:clock@link.mozilla.org","properties":{"model":"Mozilla clock v1"},"setters":{},"tags":[]}]"#;

        assert_eq!(body, s);
    }

    it "should return the list of services from a POST request" {
        let response = request::post("http://localhost:3000/api/v1/services",
                                    Headers::new(),
                                    r#"[{"id":"service:clock@link.mozilla.org"}]"#,
                                    &mount).unwrap();
        let body = response::extract_body_to_string(response);
        let s = r#"[{"adapter":"clock@link.mozilla.org","getters":{"getter:timeofday.clock@link.mozilla.org":{"adapter":"clock@link.mozilla.org","id":"getter:timeofday.clock@link.mozilla.org","kind":"CurrentTimeOfDay","mechanism":"getter","service":"service:clock@link.mozilla.org","tags":[]},"getter:timestamp.clock@link.mozilla.org":{"adapter":"clock@link.mozilla.org","id":"getter:timestamp.clock@link.mozilla.org","kind":"CurrentTime","mechanism":"getter","service":"service:clock@link.mozilla.org","tags":[]}},"id":"service:clock@link.mozilla.org","properties":{"model":"Mozilla clock v1"},"setters":{},"tags":[]}]"#;

        assert_eq!(body, s);
    }
}

#[cfg(test)]
describe! binary_getter {
    it "should return an image/png http response" {
        extern crate serde_json;

        use foxbox_taxonomy::adapter::*;
        use foxbox_taxonomy::api::{ Error, InternalError };
        use foxbox_taxonomy::manager::AdapterManager;
        use foxbox_taxonomy::services::*;
        use foxbox_taxonomy::values::{ Range, Type, Value, Binary };
        use iron::Headers;
        use iron::headers::{ ContentLength, ContentType };
        use iron_test::{ request, response };
        use mount::Mount;
        use std::collections::{ HashMap, HashSet };
        use std::sync::Arc;
        use stubs::controller::ControllerStub;
        use transformable_channels::mpsc::*;

        let taxo_manager = AdapterManager::new();

        // Create a basic adpater and service with a getter returning binary data.

        static ADAPTER_NAME: &'static str = "Test adapter";
        static ADAPTER_VENDOR: &'static str = "team@link.mozilla.org";
        static ADAPTER_VERSION: [u32;4] = [0, 0, 0, 0];

        struct BinaryAdapter { }

        impl Adapter for BinaryAdapter {
            fn id(&self) -> Id<AdapterId> {
                adapter_id!("adapter@test")
            }

            fn name(&self) -> &str {
                ADAPTER_NAME
            }

            fn vendor(&self) -> &str {
                ADAPTER_VENDOR
            }

            fn version(&self) -> &[u32;4] {
                &ADAPTER_VERSION
            }

            fn fetch_values(&self, mut set: Vec<Id<Getter>>) -> ResultMap<Id<Getter>, Option<Value>, Error> {
                set.drain(..).map(|id| {
                    if id == Id::new("getter:binary@link.mozilla.org") {
                        let vec = vec![1, 2, 3, 10, 11, 12];
                        let binary = Binary {
                            data: Arc::new(vec),
                            mimetype: Id::new("image/png")
                        };
                        return (id.clone(), Ok(Some(Value::Binary(binary))));
                    }

                    (id.clone(), Err(Error::InternalError(InternalError::NoSuchGetter(id))))
                }).collect()
            }

            fn send_values(&self, mut values: HashMap<Id<Setter>, Value>) -> ResultMap<Id<Setter>, (), Error> {
                values.drain().map(|(id, _)| {
                    (id.clone(), Err(Error::InternalError(InternalError::NoSuchSetter(id))))
                }).collect()
            }

            fn register_watch(&self, mut watch: Vec<(Id<Getter>, Option<Range>)>,
                _: Box<ExtSender<WatchEvent>>) ->
                   ResultMap<Id<Getter>, Box<AdapterWatchGuard>, Error>
            {
                watch.drain(..).map(|(id, _)| {
                    (id.clone(), Err(Error::GetterDoesNotSupportWatching(id)))
                }).collect()
            }
        }

        impl BinaryAdapter {
            fn init<A: AdapterManagerHandle>(adapt: &A) -> Result<(), Error> {
                try!(adapt.add_adapter(Box::new(BinaryAdapter { })));
                let service_id = service_id!("service@test");
                let adapter_id = adapter_id!("adapter@test");
                try!(adapt.add_service(Service::empty(service_id.clone(), adapter_id.clone())));
                try!(adapt.add_getter(Channel {
                    tags: HashSet::new(),
                    adapter: adapter_id.clone(),
                    id: Id::new("getter:binary@link.mozilla.org"),
                    last_seen: None,
                    service: service_id.clone(),
                    mechanism: Getter {
                        kind: ChannelKind::Extension {
                            vendor: Id::new(ADAPTER_VENDOR),
                            adapter: Id::new(ADAPTER_NAME),
                            kind: Id::new("binary_getter"),
                            typ: Type::Binary,
                        },
                        updated: None
                    }
                }));

                Ok(())
            }
        }

        BinaryAdapter::init(&taxo_manager).unwrap();

        let mut mount = Mount::new();
        mount.mount("/api/v1", create(ControllerStub::new(), taxo_manager));

        let response = request::put("http://localhost:3000/api/v1/channels/get",
                                    Headers::new(),
                                    r#"[{"id":"getter:binary@link.mozilla.org"}]"#,
                                    &mount).unwrap();

        let content_length = format!("{}", response.headers.get::<ContentLength>().unwrap());
        let content_type = format!("{}", response.headers.get::<ContentType>().unwrap());
        assert_eq!(content_length, "6");
        assert_eq!(content_type, "image/png");

        let result = response::extract_body_to_bytes(response);
        assert_eq!(result, vec![1, 2, 3, 10, 11, 12]);
    }
}
