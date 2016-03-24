/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

use foxbox_taxonomy::manager::WatchGuard;
use foxbox_taxonomy::api::{ API, TargetMap };
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
use std::sync::Mutex;
use traits::Controller;

/// This is a specialized Router for the taxonomy API.
/// It handles all the calls under the api/v1/ url space.
pub struct TaxonomyRouter<A> {
    api: Mutex<A>
}

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

    fn build_response<S: ToJSON>(&self, obj: &S) -> IronResult<Response> {
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
}

impl<A> Handler for TaxonomyRouter<A>
    where A: API<WatchGuard=WatchGuard> + 'static {

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
        payload_api!(fetch_values, Vec<GetterSelector>, ["channels", "get"], Method::Get);
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

        use serde_json::value::Value as JSON;
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
