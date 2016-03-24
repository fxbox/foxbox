/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use adapters::philips_hue::http;
use adapters::philips_hue::hub_api::HubApi;
use serde_json;

#[derive(Serialize, Deserialize, Debug)]
struct NupnpEntry {
    id: String,
    internalipaddress: String
}

pub fn query(server_url: &str) -> Vec<HubApi> {
    // "[{\"id\":\"001788fffe243755\",\"internalipaddress\":\"192.168.5.129\"}]"
    debug!("Querying NUPnP server at {}", server_url);
    let empty_list = Vec::new();
    let hub_list = http::get(server_url)
        .map(parse_response)
        .unwrap_or(empty_list);
    debug!("Parsed NUPnP response: {:?}", hub_list);
    hub_list
}

fn parse_response(content: String) -> Vec<HubApi> {
    let mut hub_list: Vec<HubApi> = Vec::new();
    let hubs: Vec<NupnpEntry> = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(error) => {
            warn!("Unable to parse NUPnP response: {}", error.to_string());
            Vec::<NupnpEntry>::new()
        }
    };
    for hub in hubs {
        let new_hub = HubApi::new(&hub.id, &hub.internalipaddress);
        hub_list.push(new_hub);
    }
    hub_list
}
