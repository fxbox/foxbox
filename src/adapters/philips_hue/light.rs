/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use adapters::philips_hue::hub_api::HubApi;
use adapters::philips_hue::hub_api::structs;

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
pub struct Light {
    pub hub_id: String, // TODO: move to HubApi reference
    pub hub_ip: String,
    pub hue_id: String,
}

impl Light {
    pub fn new(hub_id: &str, hub_ip: &str, light_id: &str) -> Light {
        Light {
            hub_id: hub_id.to_owned(),
            hub_ip: hub_ip.to_owned(),
            hue_id: light_id.to_owned(),
        }
    }

    pub fn get_settings(&self) -> structs::SettingsLightEntry {
        HubApi::new(&self.hub_id, &self.hub_ip)
            .get_light_status(&self.hue_id)
    }

    pub fn get_unique_id(&self) -> String {
        self.get_settings().uniqueid
    }

    pub fn get_state(&self) -> LightState {
        // TODO: Work with api reference instead of instantiating one ad-hoc.
        // Created issues in muti-treading.
        let api = HubApi::new(&self.hub_id, &self.hub_ip);
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
        let api = HubApi::new(&self.hub_id, &self.hub_ip);
        api.set_light_color(&self.hue_id, hue_hue, hue_sat, hue_val, hue_on);
    }
}
