// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! API structures used by the Philips Hue API
//!
//! This module implements various data structures that are
//! used in communicating with Philips Hue hubs. It also implements
//! JSON parsing for these strucst.

use core::fmt::Debug;
use serde::de::Deserialize;
use serde_json;
use std::collections::BTreeMap;

#[derive(Deserialize, Debug)]
pub struct Settings {
    pub config: SettingsConfig,
    pub scenes: BTreeMap<String, serde_json::Value>,
    pub lights: BTreeMap<String, SettingsLightEntry>,
    pub sensors: BTreeMap<String, serde_json::Value>,
    pub rules: BTreeMap<String, serde_json::Value>,
    pub schedules: BTreeMap<String, serde_json::Value>,
    pub groups: BTreeMap<String, serde_json::Value>,
    pub resourcelinks: Option<BTreeMap<String, serde_json::Value>>,
}

#[derive(Deserialize, Debug)]
pub struct SettingsConfig {
    pub whitelist: Option<BTreeMap<String, SettingsConfigWhitelistEntry>>,
    pub portalconnection: String,
    pub modelid: String,
    pub proxyport: u32,
    pub linkbutton: bool,
    pub dhcp: bool,
    pub factorynew: bool,
    pub zigbeechannel: u32,
    pub swupdate: BTreeMap<String, serde_json::Value>,
    pub mac: String,
    pub bridgeid: String,
    pub ipaddress: String,
    pub swversion: String,
    pub apiversion: String,
    #[serde(rename="UTC")]
    pub utc: String,
    pub localtime: String,
    pub portalstate: BTreeMap<String, serde_json::Value>,
    pub portalservices: bool,
    pub proxyaddress: String,
    pub name: String,
    pub replacesbridgeid: serde_json::Value,
    pub timezone: String,
    pub gateway: String,
    pub netmask: String,
    pub backup: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct SettingsConfigWhitelistEntry {
    pub name: String,
    #[serde(rename="create date")]
    pub create_date: String,
    #[serde(rename="last use date")]
    pub last_use_date: String,
}

#[derive(Deserialize, Debug)]
pub struct SettingsLightEntry {
    pub swversion: String,
    pub modelid: String,
    pub name: String,
    pub uniqueid: String,
    #[serde(rename="type")]
    pub lighttype: String,
    pub pointsymbol: Option<BTreeMap<String, String>>,
    pub manufacturername: String,
    pub state: SettingsLightState,
}

#[derive(Deserialize, Debug)]
pub struct SettingsLightState {
    pub on: bool,
    pub ct: Option<u32>,
    pub reachable: bool,
    pub effect: Option<String>,
    pub sat: Option<u32>,
    pub bri: u32,
    pub colormode: Option<String>,
    pub hue: Option<u32>,
    pub xy: Option<Vec<f32>>,
    pub alert: String,
}

impl Settings {
    pub fn new(json: &str) -> Option<Settings> {
        parse_json(json)
    }
}

#[allow(dead_code)]
impl SettingsLightEntry {
    pub fn new(json: &str) -> Option<Self> {
        parse_json(json)
    }
}

pub fn parse_json<T: Deserialize + Debug>(json: &str) -> Option<T> {
    let parsed: Option<T> = match serde_json::from_str(&json) {
        Ok(value) => Some(value),
        Err(error) => {
            error!("Unable to parse JSON {}. Error: {}",
                   json,
                   error.to_string());
            None
        }
    };
    debug!("Parsed JSON result: {:?}", parsed);
    parsed
}

#[cfg(test)]
describe! philips_hue_struct {

    before_each {
        let json = r#"{"state":
        {"on":true,"bri":0,"hue":0,"sat":0,"effect":"none",
        "xy":[0.0000,0.0000],"ct":0,"alert":"none","colormode":"hs",
        "reachable":false}, "type": "Extended color light",
        "name": "Hue color lamp 1", "modelid": "LCT007",
        "manufacturername": "Philips","uniqueid":"00:17:88:01:10:34:f0:0a-0c",
        "swversion": "66014919", "pointsymbol": { "1":"none", "2":"none",
        "3":"none", "4":"none", "5":"none", "6":"none", "7":"none", "8":"none"
        }}"#;
    }

    it "should parse a good SettingsLightEntry" {
        let res: SettingsLightEntry = parse_json(json).unwrap();
        assert_eq!(res.state.on, true);
    }

}
