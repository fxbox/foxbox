/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate mio;

use service::ServiceID;

pub enum EventData {
    AdapterStart { name: String },
    ServiceStart { id: ServiceID },
    ServiceStop { id: ServiceID }
}

impl EventData {
    pub fn description(&self) -> String {
        let description = match *self {
            EventData::AdapterStart { ref name } => name,
            EventData::ServiceStart { ref id } => id,
            EventData::ServiceStop { ref id } => id
        };

        description.to_string()
    }
}

pub type EventSender = mio::Sender<EventData>;
