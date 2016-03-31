/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

use foxbox_taxonomy::manager::*;
use foxbox_taxonomy::serialize::*;
use foxbox_taxonomy::api::{ API, Error, TargetMap, User };
use foxbox_taxonomy::values::Value;
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
use std::sync::Arc;
use traits::Controller;

/// This is a specialized Router for the taxonomy API.
/// It handles all the calls under the api/v1/ url space.
pub struct TaxonomyRouter {
    api: Arc<AdapterManager>
}

type GetterResultMap = ResultMap<Id<Getter>, Option<Value>, Error>;

impl TaxonomyRouter {
    pub fn new(adapter_api: &Arc<AdapterManager>) -> Self {
        TaxonomyRouter {
            api: adapter_api.clone()
        }
    }

    fn build_response<S: ToJSON>(&self, obj: S) -> IronResult<Response> {
        // Serialize the data.
        let mut bin = MultiPart::new();
        let json = obj.to_json(&mut bin);
        let serialized = itry!(serde_json::to_string(&json));

        let mut response;
        if bin.buf.is_empty() {
            // Single part.
            response = Response::with(serialized);
            response.headers.set(ContentType::json());
        } else {
            use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
            use rustc_serialize::base64;
            use rustc_serialize::base64::ToBase64;
            // Multipart

            // Generate content
            // FIXME: Create id text/json once and for all.
            bin.buf.insert(0, (Id::new("text/json"), serialized.into_bytes()));
            let mut content = "This is a response with multiple parts in MIME format.".to_owned();
            let config = base64::Config {
                char_set: base64::CharacterSet::Standard,
                newline: base64::Newline::LF,
                pad: false,
                line_length: Some(1024),
            };
            for (mime, data) in bin.buf {
                content.push_str("\n--frontier\n");
                content.push_str("\nContentType: ");
                content.push_str(&mime.to_string() as &str);
                content.push_str("\n");
                content.push_str(&data.to_base64(config) as &str);
                content.push_str("\n");
            }
            content.push_str("\n--frontier--\n");

            response = Response::with(content);
            response.headers.set(ContentType(Mime(
                TopLevel::Multipart,
                SubLevel::Ext("mixed".to_owned()),
                vec![(Attr::Boundary, Value::Ext("frontier".to_owned()))]
            )));
        }

        response.status = Some(Status::Ok);
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

}

impl Handler for TaxonomyRouter {

    #[allow(cyclomatic_complexity)]
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
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
                    if req.method != Method::Get && req.method != Method::Post {
                        // Wrong method, return a 405 error.
                        return Ok(Response::with((Status::MethodNotAllowed,
                                                  format!("Bad method: {}", req.method))));
                    }

                    if req.method == Method::Get {
                        // On a GET, just send the full taxonomy content for
                        // this kind of selector.
                        let result = self.api.$call(vec![$sel::new()]);
                        self.build_response(&result)
                    } else {
                        let source = itry!(Self::read_body_to_string(&mut req.body));
                        match Vec::<$sel>::from_str(&source as &str) {
                            Ok(arg) => self.build_response(&self.api.$call(arg)),
                            Err(err) => self.build_parse_error(&err)
                        }
                    }
                }
            })
        }

        macro_rules! simple {
            ($api:ident, $arg:ident, $call:ident) => (self.build_response(&$api.$call($arg, User::None)))
        }

        // Generates the code to process a given HTTP call with a json body.
        macro_rules! payload_api {
            ($call:ident, $param:ty, $path:expr, $method:expr, $action:ident) => (
                if path == $path && req.method == $method {
                    type Selectors = $param;
                    return {
                        let api = &self.api;
                        let source = itry!(Self::read_body_to_string(&mut req.body));
                        match Selectors::from_str(&source as &str) {
                            Ok(arg) => $action!(api, arg, $call),
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
                        self.build_response(&self.api.$call(arg_1, arg_2))
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
        payload_api!(fetch_values, Vec<GetterSelector>, ["channels", "get"], Method::Put, simple);
        payload_api!(send_values, TargetMap<SetterSelector, Value>, ["channels", "set"], Method::Put, simple);

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

pub fn create<T>(controller: T, adapter_api: &Arc<AdapterManager>) -> Chain
    where T: Controller {
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
        use std::sync::Arc;

        let taxo_manager = Arc::new(AdapterManager::new());
        clock::Clock::init(&taxo_manager).unwrap();

        let mut mount = Mount::new();
        mount.mount("/api/v1", create(ControllerStub::new(), &taxo_manager));
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

        let taxo_manager = Arc::new(AdapterManager::new());

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
            fn init(adapt: &Arc<AdapterManager>) -> Result<(), Error> {
                try!(adapt.add_adapter(Arc::new(BinaryAdapter { })));
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
        mount.mount("/api/v1", create(ControllerStub::new(), &taxo_manager));

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
