/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate hyper;
extern crate time;
extern crate url;

use foxbox_taxonomy::api::{ Error, InternalError };
use foxbox_taxonomy::services::*;
use self::hyper::header::{ Authorization, Basic, Connection };
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::io::{ BufWriter, ErrorKind };
use std::io::prelude::*;
use std::path::Path;

// TODO: The camera username and password need to be persisted per-camera
static CAMERA_USERNAME: &'static str = "admin";
static CAMERA_PASSWORD: &'static str = "password";

pub fn create_service_id(service_id: &str) -> Id<ServiceId> {
    Id::new(&format!("service:{}@link.mozilla.org", service_id))
}

pub fn create_setter_id(operation: &str, service_id: &str) -> Id<Setter> {
    create_io_mechanism_id("setter", operation, service_id)
}

pub fn create_getter_id(operation: &str, service_id: &str) -> Id<Getter> {
    create_io_mechanism_id("getter", operation, service_id)
}

pub fn create_io_mechanism_id<IO>(prefix: &str, operation: &str, service_id: &str) -> Id<IO>
    where IO: IOMechanism
{
    Id::new(&format!("{}:{}.{}@link.mozilla.org", prefix, operation, service_id))
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
            warn!("GET on {} failed: {}", url, err);
            return Err(Error::InternalError(InternalError::InvalidInitialService));
        }
    };

    if res.status != self::hyper::status::StatusCode::Ok {
        warn!("GET on {} failed: {}", url, res.status);
        return Err(Error::InternalError(InternalError::InvalidInitialService));
    }

    let mut image = Vec::new();
    match res.read_to_end(&mut image) {
        Ok(_) => Ok(image),
        Err(err) => {
            warn!("read of image data from {} failed: {}", url, err);
            Err(Error::InternalError(InternalError::InvalidInitialService))
        }
    }
}

#[derive(Clone)]
pub struct IpCamera {
    pub udn: String,
    url: String,
    snapshot_dir: String,

    pub image_list_id: Id<Getter>,
    pub image_newest_id: Id<Getter>,
    pub snapshot_id: Id<Setter>,
}

impl IpCamera {
    pub fn new(udn: &str, url: &str, root_snapshot_dir: &str) -> Result<Self, Error> {
        let camera = IpCamera {
            udn: udn.to_owned(),
            url: url.to_owned(),
            image_list_id: create_getter_id("image_list", &udn),
            image_newest_id: create_getter_id("image_newest", &udn),
            snapshot_id: create_setter_id("snapshot", &udn),
            snapshot_dir: format!("{}/{}", root_snapshot_dir, udn)
        };
        // Create a directory to store snapshots for this camera.
        if let Err(err) = fs::create_dir_all(&camera.snapshot_dir) {
            if err.kind() != ErrorKind::AlreadyExists {
                error!("Unable to create directory {}: {}", camera.snapshot_dir, err);
                return Err(Error::InternalError(InternalError::GenericError(format!("cannot create {}", camera.snapshot_dir))));
            }
        }
        Ok(camera)
    }

    pub fn get_image_list(&self) -> Vec<String> {
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
        array
    }

    pub fn get_image(&self, filename: &str) -> Result<Vec<u8>, Error> {
        let full_filename = format!("{}/{}", self.snapshot_dir, filename);
        debug!("get_image: filename = {}", full_filename.clone());
        let mut options = fs::OpenOptions::new();
        options.read(true);
        if let Ok(mut image_file) = options.open(full_filename.clone()) {
            let mut image = Vec::new();
            if let Ok(_) = image_file.read_to_end(&mut image) {
                return Ok(image);
            }
            warn!("Error reading {}", full_filename);
        } else {
            warn!("Image {} not found", full_filename);
        }
        Err(Error::InternalError(InternalError::InvalidInitialService))
    }

    pub fn get_newest_image(&self) -> Result<Vec<u8>, Error> {
        let mut newest_image_time = 0;
        let mut newest_image = None;
        if let Ok(iter) = fs::read_dir(Path::new(&self.snapshot_dir)) {
            for entry in iter {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            let time = metadata.ctime();
                            if newest_image_time <= time {
                                newest_image_time = time;
                                newest_image = Some(String::from(entry.file_name().to_str().unwrap()));
                            }
                        }
                    }
                }
            }
        }

        if newest_image.is_none() {
            return Err(Error::InternalError(InternalError::InvalidInitialService));
        }
        self.get_image(&newest_image.unwrap())
    }

    pub fn take_snapshot(&self) -> Result<String, Error> {
        let image_url = "image/jpeg.cgi";
        let url = format!("{}/{}", self.url, image_url);

        let image = match get_bytes(url) {
            Ok(image) => image,
            Err(err) => {
                warn!("Error '{:?}' retrieving image from camera {}", err, self.url);
                return Err(Error::InternalError(InternalError::InvalidInitialService));
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
                    warn!("Unable to open {}: {:?}", full_filename, err.kind());
                    return Err(Error::InternalError(InternalError::InvalidInitialService));
                }
            };

            break;
        }
        let mut writer = BufWriter::new(&image_file);
        match writer.write_all(&image) {
            Ok(_) => {}
            Err(err) => {
                warn!("Error '{:?}' writing snapshot.jpg for camera {}", err, self.udn);
                return Err(Error::InternalError(InternalError::InvalidInitialService));
            }
        }
        debug!("Took a snapshot from {}: {}", self.udn, full_filename);
        Ok(format!("{}.jpg", filename))
    }
}

