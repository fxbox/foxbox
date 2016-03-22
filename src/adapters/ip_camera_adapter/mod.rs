/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate time;
extern crate url;

use iron::{ Request, Response, IronResult };
use iron::headers::{ ContentType, AccessControlAllowOrigin };
use iron::status::Status;
use router::Router;
use service::{ Service, ServiceAdapter, ServiceProperties };
use std::collections::BTreeMap;
use std::fs;
use std::io::{ BufWriter, Error, ErrorKind };
use std::io::prelude::*;
use std::path::Path;
use std::sync::Arc;
use upnp::{ UpnpListener, UpnpService };
use traits::Controller;

use self::hyper::header::{ Authorization, Basic, Connection };
use self::url::{ Host, Url };

// TODO: The camera username and password need to be persisted per-camera
static CAMERA_USERNAME: &'static str = "admin";
static CAMERA_PASSWORD: &'static str = "password";
static SNAPSHOT_DIR: &'static str = "snapshots";

const CUSTOM_PROPERTY_MANUFACTURER: &'static str = "manufacturer";
const CUSTOM_PROPERTY_TYPE: &'static str = "type";
const CUSTOM_PROPERTY_MODEL: &'static str = "model";

const SERVICE_TYPE: &'static str = "ipcamera";
const FILENAME_QUERY_PARAMETER_NAME: &'static str = "filename";

fn response_json(json_str: String) -> IronResult<Response> {
    let mut response = Response::with(json_str);
    response.status = Some(Status::Ok);
    response.headers.set(AccessControlAllowOrigin::Any);
    response.headers.set(ContentType::json());
    Ok(response)
}

macro_rules! error_response {
    ( $x:expr ) => {{   // error_response!("Foo")
        error!($x);
        response_json(json!({ error: $x }))
    }};

    ( $($x:expr),* ) => {{  // error_reponse!("Some arg: {}", 123)
        let err_str = format!( $( $x ),* );
        error!("{}", err_str);
        response_json(json!({ error: err_str }))
    }}
}

macro_rules! success_response {
    ( $filename:expr, $($x:expr),* ) => {{  // success_response!(filename, "Some arg: {}", 123)
        let success_str = format!( $( $x ),* );
        info!("{}", success_str);
        response_json(json!({ success: success_str, filename: $filename }))
    }}
}

fn get_bytes(url: String) -> Result<Vec<u8>, Error> {
    let client = hyper::Client::new();
    let get_result = client.get(&url)
                           .header(
                               Authorization(
                                   Basic {
                                       username: CAMERA_USERNAME.to_owned(),
                                       password: Some(CAMERA_PASSWORD.to_owned())
                                   }
                               )
                           )
                           .header(Connection::close())
                           .send();
    let mut res = match get_result {
        Ok(res) => res,
        Err(err) => {
            return Err(Error::new(ErrorKind::Other,
                                  format!("GET on {} failed: {}", url, err)));
        }
    };

    if res.status != Status::Ok {
        debug!("ip_camera_adapter: get_bytes {}", res.status);
        return Err(Error::new(ErrorKind::Other,
                              format!("GET on {} failed: {}", url, res.status)));
    }

    let mut image = Vec::new();
    match res.read_to_end(&mut image) {
        Ok(_) => Ok(image),
        Err(err) => Err(Error::new(ErrorKind::Other,
                                   format!("read of image data from {} failed: {}", url, err)))
    }
}

struct IpCameraService<T> {
    controller: T,
    properties: ServiceProperties,
    ip: String,
    snapshot_dir: String,
    name: String,
}

impl<T: Controller> IpCameraService<T> {
    fn new(controller: T, id: &str, ip: &str, name: &str, properties: BTreeMap<String, String>)
           -> Self {
        debug!("Creating IpCameraService");
        IpCameraService {
            controller: controller.clone(),
            properties: ServiceProperties {
                id: id.to_owned(),
                name: "IpCameraService".to_owned(),
                description: format!("IP Camera: {}", name).to_owned(),
                http_url: controller.get_http_root_for_service(id.to_owned()),
                ws_url: controller.get_ws_root_for_service(id.to_owned()),
                custom_properties: properties,
            },
            ip: ip.to_owned(),
            snapshot_dir: format!("{}/{}", controller.get_profile().path_for(SNAPSHOT_DIR), id),
            name: name.to_owned(),
        }
    }

    fn cmd_get(&self, filename: &str) -> IronResult<Response> {
        let full_filename = format!("{}/{}", self.snapshot_dir, filename);
        debug!("cmd_get: filename = {}", full_filename.clone());
        let mut options = fs::OpenOptions::new();
        options.read(true);
        if let Ok(mut image_file) = options.open(full_filename.clone()) {
            let mut image = Vec::new();
            if let Ok(_) = image_file.read_to_end(&mut image) {
                let mut response = Response::with(image);
                response.status = Some(Status::Ok);
                response.headers.set(AccessControlAllowOrigin::Any);
                response.headers.set(ContentType::jpeg());
                return Ok(response);
            }
            return error_response!("Error reading {}", full_filename);
        }

        error_response!("Image {} not found", full_filename)
    }

    fn cmd_list(&self) -> IronResult<Response> {
        let mut array: Vec<String> = vec!();
        if let Ok(iter) = fs::read_dir(Path::new(&self.snapshot_dir)) {
            for entry in iter {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            array.push(String::from(entry.file_name().to_str().unwrap()));
                        }
                    }
                }
            }
        }
        let serialized = itry!(serde_json::to_string(&array));
        let mut response = Response::with(serialized);
        response.status = Some(Status::Ok);
        response.headers.set(AccessControlAllowOrigin::Any);
        response.headers.set(ContentType::json());
        Ok(response)
    }

    fn cmd_snapshot(&self) -> IronResult<Response> {
        let image_url = "image/jpeg.cgi";
        let url = format!("http://{}/{}", self.ip, image_url);

        let image = match get_bytes(url) {
            Ok(image) => image,
            Err(err) => {
                return error_response!("Error '{:?}' retrieving image from camera ip {}", err, self.ip);
            }
        };

        let mut options = fs::OpenOptions::new();
        options.write(true);
        options.create(true);
        options.truncate(true);

        let filename_base = time::strftime("%Y-%m-%d-%H%M%S", &time::now()).unwrap();
        let mut full_filename;
        let image_file;
        let mut loop_count = 0;
        let mut filename;
        loop {
            if loop_count == 0 {
                filename = filename_base.clone();
            } else {
                filename = format!("{}-{}", filename_base, loop_count);
            }
            full_filename = format!("{}/{}.jpg", self.snapshot_dir, filename);

            if fs::metadata(full_filename.clone()).is_ok() {
                // File exists
                loop_count += 1;
                continue;
            }

            image_file = match options.open(full_filename.clone()) {
                Ok(file) => file,
                Err(err) => {
                    return error_response!("Unable to open {}: {:?}", full_filename, err.kind());
                }
            };

            break;
        }
        let mut writer = BufWriter::new(&image_file);
        match writer.write_all(&image) {
            Ok(_) => {}
            Err(err) => {
                return error_response!("Error '{:?}' writing snapshot.jpg for camera {}", err, self.name);
            }
        }
        success_response!(format!("{}.jpg", filename), "Took a snapshot from {}: {}", self.name, full_filename)
    }
}

impl<T: Controller> Service for IpCameraService<T> {
    fn get_properties(&self) -> ServiceProperties {
        self.properties.clone()
    }

    fn start(&self) {
        let props = self.properties.clone();
        let controller = self.controller.clone();

        let model_name = props.custom_properties.get(CUSTOM_PROPERTY_MODEL).unwrap();

        info!("Starting IpCamera {} Model: {} Name: {}", props.id, model_name, self.name);

        // Create a directory to store snapshots for this camera.
        match fs::create_dir_all(&self.snapshot_dir) {
            Ok(_) => {},
            Err(err) => {
                error!("Unable to create directory {}: {}", self.snapshot_dir, err);
                controller.remove_service(props.id);
            }
        }
    }

    fn stop(&self) {
        let props = self.properties.clone();
        info!("Stopping IpCameraService for ID: {} Name: {}", props.id, self.name);
    }

    // Processes a http request.
    fn process_request(&self, req: &mut Request) -> IronResult<Response> {
        let cmd = req.extensions.get::<Router>().unwrap().find("command").unwrap_or("");
        info!("IpCameraAdapter {:?} received command {} for camera ip {}", req, cmd, self.ip);

        match cmd {
            "snapshot" => self.cmd_snapshot(),
            "list" => self.cmd_list(),
            "get" => {
                if let Some(query) = req.url.query.as_ref() {
                    // Creating a fake URL to get the query parsed.
                    let url = Url::parse(&format!("http://box.fox?{}", query)).unwrap();
                    if let Some(pairs) = url.query_pairs() {
                        let filename =  pairs.iter()
                            .find(|ref set| set.0.to_lowercase() == FILENAME_QUERY_PARAMETER_NAME)
                            .map(|ref set| set.1.clone());

                        if let Some(filename) = filename {
                            return self.cmd_get(&filename);
                        }
                    }
                }

                error_response!("`get` command needs a `{}` query string parameter",
                                FILENAME_QUERY_PARAMETER_NAME)
            },
            _ => error_response!("Unrecognized command: {}", cmd)
        }
    }
}

struct IpCameraUpnpListener<T> {
    controller: T
}

impl<T: Controller> IpCameraUpnpListener<T> {
    pub fn new(controller: T) -> Arc<Self> {
        Arc::new(IpCameraUpnpListener {
            controller: controller.clone()
        })
    }
}

impl<T: Controller> UpnpListener for IpCameraUpnpListener<T> {

    // This will called each time that the device advertises itself using UPNP.
    // The D-Link cameras post an advertisement once when we do our search
    // (when the adapter is started) and 4 times in a row about once every
    // 3 minutes when they're running.
    fn upnp_discover(&self, service: &UpnpService) -> bool {

        macro_rules! try_get {
            ($hash:expr, $key:expr) => (match $hash.get($key) {
                Some(val) => val,
                None => return false
            })
        }

        let model_name = try_get!(service.description, "/root/device/modelName");
        let known_models = [
            "DCS-5010L", "DCS-5020L", "DCS-5025L"
        ];
        let model_name_str: &str = &model_name;
        if !known_models.contains(&model_name_str) {
            return false;
        }

        let url = try_get!(service.description, "/root/device/presentationURL");
        // Extract the IP address from the presentation URL, which will look
        // something like: http://192.168.1.123:80
        let ip;
        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(&Host::Ipv4(host_ip)) = parsed_url.host() {
                ip = host_ip.to_string();
            } else {
                return false;
            }
        } else {
            return false
        }

        let mut udn = try_get!(service.description, "/root/device/UDN").clone();
        // The UDN is typically of the for uuid:SOME-UID-HERE, but some devices
        // response with just a UUID. We strip off the uuid: prefix, if it exists
        // and use the resulting UUID as the service id.
        if udn.starts_with("uuid:") {
            udn = String::from(&udn[5..]);
        }

        // Since the upnp_discover will be called about once wvery 3 minutes
        // we want to ignore discoveries if the camera is already registered.

        // TODO: We really need to update the IP/camera name in the event that
        //       it changed. I'll add this once we start persisting the camera
        //       information in a database.
        if let Some(_) = self.controller.get_service_properties(udn.clone()) {
            debug!("Found {} @ {} UDN {} (ignoring since it already exists)", model_name, ip, udn);
            return true;
        }

        let name = try_get!(service.description, "/root/device/friendlyName").clone();
        let manufacturer = try_get!(service.description, "/root/device/manufacturer");

        debug!("Adding IpCamera {} Manufacturer: {} Model: {} Name: {}", udn, manufacturer,
               model_name, name);

        let mut custom_properties = BTreeMap::<String, String>::new();
        custom_properties.insert(CUSTOM_PROPERTY_MANUFACTURER.to_owned(), manufacturer.to_owned());
        custom_properties.insert(CUSTOM_PROPERTY_TYPE.to_owned(), SERVICE_TYPE.to_owned());
        custom_properties.insert(CUSTOM_PROPERTY_MODEL.to_owned(), model_name.to_owned());

        // Create the IpCameraService.
        let service = IpCameraService::new(self.controller.clone(), &udn, &ip, &name,
                                           custom_properties);
        service.start();
        self.controller.add_service(Box::new(service));

        true
    }
}

pub struct IpCameraAdapter<T> {
    name: String,
    controller: T
}

impl<T: Controller> IpCameraAdapter<T> {
    pub fn new(controller: T) -> Self {
        debug!("Creating IpCameraAdapter");
        IpCameraAdapter { name: "IpCameraAdapter".to_owned(),
                          controller: controller }
    }
}

impl<T: Controller> ServiceAdapter for IpCameraAdapter<T> {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn start(&self) {
        let controller = self.controller.clone();
        let listener = IpCameraUpnpListener::new(controller.clone());
        let upnp = controller.get_upnp_manager();

        // The UPNP listener will add camera service for discovered cameras
        upnp.add_listener("IpCamera".to_owned(), listener);

        // The UPNP service searches for ssdp:all which the D-Link cameras
        // don't seem to respond to. So we search for this instead, which
        // they do respond to.
        upnp.search(Some("urn:cellvision:service:Null:1".to_owned())).unwrap();
    }

    fn stop(&self) {
        debug!("Stopping IPCameraAdapter");
    }
}
