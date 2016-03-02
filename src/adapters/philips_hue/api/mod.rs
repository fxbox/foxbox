/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![allow(dead_code)]

extern crate serde_json;

pub mod structs;

use adapters::philips_hue::http;
use std;
use std::collections::BTreeMap;
use std::error::Error;
use std::hash::{ Hash, SipHasher, Hasher };

#[derive(Debug, Hash)]
struct HueHubApiToken {
    iv: String,
    id: String,
}

impl HueHubApiToken {
    // This implementation offers 64 bit
    fn as_hash(&self) -> String {
        let mut hasher = SipHasher::new();
        self.hash(&mut hasher);
        format!("{:016x}", hasher.finish()).to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct HueHubApi {
    pub id: String,
    pub ip: String,
    pub token: String,
}

impl std::fmt::Display for HueHubApi {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Hue Bridge id:{} at {:?}", self.id, self.ip)
    }
}

impl HueHubApi {
    pub fn new(id: String, ip: String) -> HueHubApi {
        // TODO: Generate a unique but reproducible access token.
        // The token must not be guessable by an outsider.

        // TODO: replace with a loooong stored random string
        let stored_random = String::from("4"); // chosen by fair dice roll.
        let token_gen = HueHubApiToken { iv: stored_random, id: id.clone() };
        let token = format!("foxbox-{}", token_gen.as_hash());
        HueHubApi { id: id, ip: ip, token: token }
    }

    pub fn get(&self, cmd: String) -> Result<String, Box<Error>> {
        let url = format!("http://{}/api/{}/{}", self.ip, self.token, cmd);
        debug!("GET request to Philips Hue bridge {}: {}", self.id, url);
        let content = http::get(url);
        debug!("Philips Hue API response: {:?}", content);
        content
    }

    pub fn post(&self, cmd: String, data: String) -> Result<String, Box<Error>> {
        let url = format!("http://{}/api/{}/{}", self.ip, self.token, cmd);
        debug!("POST request to Philips Hue bridge {}: {} data: {}", self.id, &url, &data);
        let content = http::post(url, data);
        debug!("Philips Hue API response: {:?}", content);
        content
    }

    pub fn post_unauth(&self, cmd: String, data: String) -> Result<String, Box<Error>> {
        let url = format!("http://{}/{}", self.ip, cmd);
        debug!("POST request to Philips Hue bridge {}: {} data: {}", self.id, &url, &data);
        let content = http::post(url, data);
        debug!("Philips Hue API response: {:?}", content);
        content
    }

    pub fn put(&self, cmd: String, data: String) -> Result<String, Box<Error>> {
        let url = format!("http://{}/api/{}/{}", self.ip, self.token, cmd);
        debug!("PUT request to Philips Hue bridge {}: {} data: {}", self.id, &url, &data);
        let content = http::put(url, data);
        debug!("Philips Hue API response: {:?}", content);
        content
    }

    pub fn is_available(&self) -> bool {
        let url = format!("http://{}/", self.ip);
        let content = http::get(url);
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
        self.get("".to_owned()).unwrap_or("".to_owned()) // TODO no unwrap
    }

    pub fn is_paired(&self) -> bool {
        let settings = self.get_settings();
        !settings.contains("unauthorized user")
    }

    pub fn try_pairing(&self) -> bool {
        // [{"success":{"username":"foxboxb-001788fffe25681a"}}]
        // [{"error":{"type":101,"address":"/","description":"link button not pressed"}}]
        let url = "api".to_owned();
        let req = json!({ username: self.token, devicetype: "foxbox_hub"});
        let response = self.post_unauth(url.clone(),
            req.clone()).unwrap_or("".to_owned()); // TODO: no unwrap
        response.contains("success")
    }

    pub fn get_lights(&self) -> Vec<HueLight> {
        let mut lights: Vec<HueLight> = Vec::new();
        let url = "lights".to_owned();
        let res = self.get(url).unwrap(); // TODO: remove unwrap
        let json: BTreeMap<String, structs::HueHubSettingsLightEntry> = structs::parse_json(&res).unwrap(); // TODO: no unwrap

        for (key,_) in json {
            let light = HueLight::new(self.id.clone(), self.ip.clone(), key);
            lights.push(light);
        }

        lights
    }

    pub fn get_light_status(&self, id: &String) -> structs::HueHubSettingsLightEntry {
        let url = format!("lights/{}", id);
        let res = self.get(url).unwrap(); // TODO: remove unwrap
        structs::parse_json(&res).unwrap() // TODO no unwrap
    }

    pub fn set_light_color(&self, light_id: &String, hue: u32, sat: u32, val: u32, on: bool) {
        let url = format!("lights/{}/state", light_id);
        let cmd = json!({ hue: hue, sat: sat, bri: val, on: on });
        let _ = self.put(url, cmd);
    }

}

#[derive(Debug)]
pub struct LightState {
    pub hue: f32,
    pub sat: f32,
    pub val: f32,
    pub on: bool,
}

impl LightState {
    pub fn new(hue: f32, sat: f32, val: f32, on: bool) -> LightState {
        LightState { hue: hue, sat: sat, val: val, on: on }
    }
}

#[derive(Debug, Clone)]
pub struct HueLight {
    pub hub_id: String, // TODO: move to HueHubApi reference
    pub hub_ip: String,
    pub hue_id: String,
}

impl HueLight {
    pub fn new(hub_id: String, hub_ip: String, light_id: String) -> HueLight {
        HueLight {
            hub_id: hub_id,
            hub_ip: hub_ip,
            hue_id: light_id,
        }
    }

    pub fn get_settings(&self) -> structs::HueHubSettingsLightEntry {
        HueHubApi::new(self.hub_id.clone(), self.hub_ip.clone())
            .get_light_status(&self.hue_id)
    }

    pub fn get_unique_id(&self) -> String {
        self.get_settings().uniqueid
    }

    pub fn get_state(&self) -> LightState {
        // TODO: Work with api reference instead of instantiating one ad-hoc.
        // Created issues in muti-treading.
        let api = HueHubApi::new(self.hub_id.clone(), self.hub_ip.clone());
        let ls = api.get_light_status(&self.hue_id);
        let hue: f32 = ls.state.hue as f32 / 65535f32 * 360f32;
        let sat: f32 = ls.state.sat as f32 / 254f32;
        let val: f32 = ls.state.bri as f32 / 254f32;
        let on = ls.state.on;

        LightState::new(hue, sat, val, on)
    }

    pub fn set_state(&self, state: LightState) {
        // Ensure valid ranges
        let hue = ((state.hue % 360f32) + 360f32) % 360f32; // [0,360)
        let mut sat = state.sat; // [0,1]
        if sat < 0f32 { sat = 0f32 };
        if sat > 1f32 { sat = 1f32 };
        let mut val = state.val; // [0,1]
        if val < 0f32 { val = 0f32 };
        if val > 1f32 { val = 1f32 };

        // convert
        let hue_hue: u32 = (hue * 65536f32 / 360f32) as u32;
        let hue_sat: u32 = (sat * 254f32) as u32;
        let hue_val: u32 = (val * 254f32) as u32;
        let hue_on = state.on;

        // TODO: Work with api reference instead of instantiating one ad-hoc.
        // Created issues in muti-treading.
        let api = HueHubApi::new(self.hub_id.clone(), self.hub_ip.clone());
        api.set_light_color(&self.hue_id, hue_hue, hue_sat, hue_val, hue_on);
    }
}
