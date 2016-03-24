/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

pub mod structs;

use adapters::philips_hue::light::Light;
use adapters::philips_hue::http;
use serde_json;
use std;
use std::collections::BTreeMap;
use std::error::Error;
use std::hash::{ Hash, SipHasher, Hasher };

#[derive(Debug, Hash)]
struct HubApiToken {
    iv: String,
    id: String,
}

impl HubApiToken {
    // This implementation offers 64 bit
    fn as_hash(&self) -> String {
        let mut hasher = SipHasher::new();
        self.hash(&mut hasher);
        format!("{:016x}", hasher.finish()).to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct HubApi {
    pub id: String,
    pub ip: String,
    pub token: String,
}

impl std::fmt::Display for HubApi {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Hue Bridge id:{} at {:?}", self.id, self.ip)
    }
}

impl HubApi {
    pub fn new(id: &str, ip: &str) -> HubApi {
        // TODO: Generate a unique but reproducible access token.
        // The token must not be guessable by an outsider.

        // TODO: replace with a loooong stored random string
        let stored_random = String::from("4"); // chosen by fair dice roll.
        let token_gen = HubApiToken { iv: stored_random, id: id.to_owned() };
        let token = format!("foxbox-{}", token_gen.as_hash());
        HubApi { id: id.to_owned(), ip: ip.to_owned(), token: token }
    }

    pub fn get(&self, cmd: &str) -> Result<String, Box<Error>> {
        let url = format!("http://{}/api/{}/{}", self.ip, self.token, cmd);
        debug!("GET request to Philips Hue bridge {}: {}", self.id, url);
        let content = http::get(&url);
        debug!("Philips Hue API response: {:?}", content);
        content
    }

    #[allow(dead_code)]
    pub fn post(&self, cmd: &str, data: &str) -> Result<String, Box<Error>> {
        let url = format!("http://{}/api/{}/{}", self.ip, self.token, cmd);
        debug!("POST request to Philips Hue bridge {}: {} data: {}", self.id, url, data);
        let content = http::post(&url, data);
        debug!("Philips Hue API response: {:?}", content);
        content
    }

    pub fn post_unauth(&self, cmd: &str, data: &str) -> Result<String, Box<Error>> {
        let url = format!("http://{}/{}", self.ip, cmd);
        debug!("POST request to Philips Hue bridge {}: {} data: {}", self.id, url, data);
        let content = http::post(&url, data);
        debug!("Philips Hue API response: {:?}", content);
        content
    }

    pub fn put(&self, cmd: &str, data: &str) -> Result<String, Box<Error>> {
        let url = format!("http://{}/api/{}/{}", self.ip, self.token, cmd);
        debug!("PUT request to Philips Hue bridge {}: {} data: {}", self.id, url, data);
        let content = http::put(&url, data);
        debug!("Philips Hue API response: {:?}", content);
        content
    }

    pub fn is_available(&self) -> bool {
        let url = format!("http://{}/", self.ip);
        let content = http::get(&url);
        match content {
            Ok(value) => {
                value.contains("hue personal wireless lighting")
            },
            Err(_) => {
                false
            }
        }
    }

    pub fn get_settings(&self) -> String {
        // [{"error":{"type":1,"address":"/","description":"unauthorized user"}}]
        self.get("").unwrap_or("".to_owned()) // TODO no unwrap
    }

    pub fn is_paired(&self) -> bool {
        let settings = self.get_settings();
        !settings.contains("unauthorized user")
    }

    pub fn try_pairing(&self) -> bool {
        // [{"success":{"username":"foxboxb-001788fffe25681a"}}]
        // [{"error":{"type":101,"address":"/","description":"link button not pressed"}}]
        let url = "api";
        let req = json!({ username: self.token, devicetype: "foxbox_hub"});
        let response = self.post_unauth(&url, &req).unwrap_or("".to_owned()); // TODO: no unwrap
        response.contains("success")
    }

    pub fn get_lights(&self) -> Vec<Light> {
        let mut lights: Vec<Light> = Vec::new();
        let url = "lights";
        let res = self.get(url).unwrap(); // TODO: remove unwrap
        let json: BTreeMap<String, structs::SettingsLightEntry> =
            structs::parse_json(&res).unwrap(); // TODO: no unwrap

        for (key, value) in json {
            if value.lighttype == "Extended color light" {
                let light = Light::new(&self.id, &self.ip, &key);
                lights.push(light);
            }
        }

        lights
    }

    pub fn get_light_status(&self, id: &str) -> structs::SettingsLightEntry {
        let url = format!("lights/{}", id);
        let res = self.get(&url).unwrap(); // TODO: remove unwrap
        structs::parse_json(&res).unwrap() // TODO no unwrap
    }

    pub fn set_light_color(&self, light_id: &str, hue: u32, sat: u32, val: u32, on: bool) {
        let url = format!("lights/{}/state", light_id);
        let cmd = json!({ hue: hue, sat: sat, bri: val, on: on });
        let _ = self.put(&url, &cmd);
    }

}
