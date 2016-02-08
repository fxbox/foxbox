/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate serde_json;

use adapters::philips_hue::http;
use adapters::philips_hue::api::HueHubApi;

#[derive(Serialize, Deserialize, Debug)]
struct HueNupnpEntry {
    id: String,
    internalipaddress: String
}

pub fn query() -> Vec<HueHubApi> {
    // "[{\"id\":\"001788fffe243755\",\"internalipaddress\":\"192.168.5.129\"}]"
    debug!("query meethue");
    let empty = String::from("[]");
    let url = "http://www.meethue.com/api/nupnp".to_owned();
    let content = http::get(url)
        .unwrap_or(empty);
    debug!("content: {:?}", content);
    let hub_list = parse_response(content);
    debug!("parsed: {:?}", hub_list);
    hub_list
}

fn parse_response(content: String) -> Vec<HueHubApi> {
    let mut hub_list: Vec<HueHubApi> = Vec::new();
    let hubs: Vec<HueNupnpEntry> = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(error) => {
            warn!("Unable to parse NUPnP response: {}", error.to_string());
            Vec::<HueNupnpEntry>::new()
        }
    };
    for hub in hubs {
        let id = hub.id;
        let ip = hub.internalipaddress;
        let new_hub = HueHubApi::new(id.to_owned(), ip.to_owned());
        hub_list.push(new_hub);
    }
    hub_list
}
