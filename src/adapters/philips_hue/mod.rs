/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

mod http;
mod nupnp;
mod api;

use adapters::philips_hue::api::structs::*;
use adapters::philips_hue::api::HueLight;
use controller::Controller;
use iron::{ Request, Response, IronResult };
use iron::headers::{ ContentType, AccessControlAllowOrigin };
use iron::status::Status;
use iron::method::Method;
use router::Router;
use service::{ Service, ServiceAdapter, ServiceProperties };
use std::time::Duration;
use std::thread;
use uuid::Uuid;
use std::io::Read;


pub struct PhilipsHueAdapter<T> {
    name: String,
    controller: T,
}

impl<T: Controller> PhilipsHueAdapter<T> {
    pub fn new(controller: T) -> Self {
        debug!("Creating Philips Hue adapter");
        PhilipsHueAdapter { name: "PhilipsHueAdapter".to_owned(),
                       controller: controller,
                     }
    }
}

impl<T: Controller> ServiceAdapter for PhilipsHueAdapter<T> {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn start(&self) {
        let mut id = 0;
        let controller = self.controller.clone();

        thread::spawn(move || {
            controller.adapter_started("Philips Hue Service Adapter".to_owned());

            let nupnp_hubs = nupnp::query();
            debug!("nUPnP reported Philips Hue bridges: {:?}", nupnp_hubs);

            for hub in nupnp_hubs {
                if !hub.is_available() {
                    continue;
                }

                // For each Hub, spawn a thread that will check availability and
                // pairing.
                let controller = controller.clone();
                thread::spawn(move || {
                    // If the Hub is not paired, try pairing.
                    if !hub.is_paired() {
                        info!("Push pairing button on Philips Hue Bridge ID {}", hub.id);

                        // Try pairing for 120 seconds.
                        for _ in 0..120 {
                            controller.adapter_notification(
                                json_value!({ adapter: "philips_hue", message: "NeedsPairing", hub: hub.id }));
                            if hub.try_pairing() {
                                break;
                            }
                            thread::sleep(Duration::from_millis(1000));
                        }

                        if hub.is_paired() {
                            info!("Paired with Philips Hue Bridge ID {}", hub.id);
                            controller.adapter_notification(
                                json_value!({ adapter: "philips_hue", message: "PairingSuccess", hub: hub.id }));
                        } else {
                            warn!("Pairing timeout with Philips Hue Bridge ID {}", hub.id);
                            controller.adapter_notification(
                                json_value!({ adapter: "philips_hue", message: "PairingTimeout", hub: hub.id }));
                            // Giving up for this Hub.
                            return;
                        }
                    }

                    // We have a paired Hub, instanciate the lights services.
                    // Extract and log some info
                    let setting = hub.get_settings();
                    let hs = HueHubSettings::new(&setting).unwrap(); // TODO: no unwrap
                    info!(
                        "Connected to Philips Hue bridge model {}, ID {}, software version {}, IP address {}",
                        hs.config.modelid, hs.config.bridgeid, hs.config.swversion,
                        hs.config.ipaddress);

                    let lights = hub.get_lights();
                    for light in lights {
                        debug!("Creating service for {:?}", light);
                        id += 1;
                        let service = HueLightService::new(controller.clone(), id, light);
                        service.start();
                        controller.add_service(Box::new(service));
                    }
                });
            }
        });
    }

    fn stop(&self) {
        info!("Stopping Philips Hue adapter");
    }

}

struct HueLightService<T> {
    controller: T,
    properties: ServiceProperties,
    light: HueLight,
}

impl<T: Controller> HueLightService<T> {
    fn new(controller: T, id: u32, light: HueLight) -> Self {
        debug!("Creating HueLightService {} for HueLight {:?}", id, light);
        let service_id = Uuid::new_v4().to_simple_string();
        HueLightService {
            controller: controller.clone(),
            properties: ServiceProperties {
                id: service_id.clone(),
                name: "philips hue service".to_owned(),
                description: "Service for Philips Hue Light".to_owned(),
                http_url: controller.get_http_root_for_service(service_id.clone()),
                ws_url: controller.get_ws_root_for_service(service_id)
            },
            light: light
        }
    }

    fn handle_get_request(&self, cmd: &str) -> IronResult<Response> {
        match cmd {
            "state" => {
                // TODO: Every light.get_*() call produces a
                // get request to the API. Fix requires major
                // internal design change.
                let status = self.light.get_settings();
                let light_state = self.light.get_state();
                let json = json!(
                    { type: "device/light/colorlight",
                      available: status.state.reachable,
                      on: status.state.on,
                      hue: light_state.hue,
                      sat: light_state.sat,
                      val: light_state.val });
                let mut response = Response::with(json);
                response.status = Some(Status::Ok);
                response.headers.set(ContentType::json());
                response.headers.set(AccessControlAllowOrigin::Any);
                Ok(response)
            },
            _ => {
                error_response()
            }
        }
    }

    fn handle_put_request(&self, cmd: &str, body: String) -> IronResult<Response> {
        match cmd {
            "state" => {
                debug!("Request body for state command: {}", body);
                let state_cmd: Option<StateCmd> = parse_json(&body);
                debug!("Parsed state command: {:?}", state_cmd);
                match state_cmd {
                    Some(value) => {
                        let mut light_state = self.light.get_state();
                        if let Some(on) = value.on { light_state.on = on; }
                        if let Some(hue) = value.hue { light_state.hue = hue; }
                        if let Some(sat) = value.sat { light_state.sat = sat; }
                        if let Some(val) = value.val { light_state.val = val; }
                        self.light.set_state(light_state);
                        success_response()
                    },
                    None => {
                        warn!("Invalid parameters in state command: {}", cmd);
                        error_response()
                    }
                }
            },
            _ => {
                warn!("Invalid command to Hue Light service: {}", cmd);
                error_response()
            }
        }
    }
}

impl<T: Controller> Service for HueLightService<T> {
    fn get_properties(&self) -> ServiceProperties {
        self.properties.clone()
    }

    // Starts the service, it will just spawn a thread and send messages once
    // in a while.
    fn start(&self) {
        info!("Service {} started for Philips Hue light \"{}\" on bridge {}",
            self.properties.id, self.light.hue_id, self.light.hub_id);
    }

    fn stop(&self) {
        debug!("Service {} stopped for Philips Hue light \"{}\" on bridge {}",
            self.properties.id, self.light.hue_id, self.light.hub_id);
    }

    // Processes a http request.
    fn process_request(&self, req: &mut Request) -> IronResult<Response> {
        let cmd = req.extensions.get::<Router>().unwrap().find("command").unwrap_or("");
        debug!("Got command {} via {:?}", cmd, req);
        match req.method {
            Method::Get => {
                self.handle_get_request(cmd)
            },
            Method::Put => {
                let mut body = String::new();
                req.body.read_to_string(&mut body).unwrap();
                self.handle_put_request(cmd, body)
            },
            _ => {
                error_response()
            }
        }
    }
}

fn success_response() -> IronResult<Response> {
    let mut response = Response::with("{\"result\": \"success\"}".to_owned());
    response.status = Some(Status::Ok);
    response.headers.set(ContentType::json());
    response.headers.set(AccessControlAllowOrigin::Any);
    Ok(response)
}

fn error_response() ->  IronResult<Response> {
    let mut response = Response::with("{\"result\": \"error\"}".to_owned());
    response.status = Some(Status::MethodNotAllowed);
    response.headers.set(ContentType::json());
    response.headers.set(AccessControlAllowOrigin::Any);
    Ok(response)
}

#[derive(Serialize, Deserialize, Debug)]
struct StateCmd {
    on: Option<bool>,
    hue: Option<f32>,
    sat: Option<f32>,
    val: Option<f32>,
}
