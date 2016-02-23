/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate mio;

use service::ServiceID;

pub enum EventData {
    AdapterStart { name: String },
    ServiceStart { id: ServiceID },
    ServiceBip { i: String, id: ServiceID, msg: String },
    ServiceStop { id: ServiceID }
}

impl EventData {
    pub fn description(&self) -> String {
        let description = match *self {
            EventData::AdapterStart { ref name } => name,
            EventData::ServiceStart { ref id }
            | EventData::ServiceStop { ref id } => id,
            EventData::ServiceBip { ref i, ref id, ref msg } => msg,
        };

        description.to_string()
    }
}

pub type EventSender = mio::Sender<EventData>;


#[cfg(test)]
describe! event_data {
    it "AdapterStart should return its name as a description" {
        let data = EventData::AdapterStart { name: "name".to_owned() };
        assert_eq!(data.description(), "name");
    }

    it "ServiceStart should return its ID as a description" {
        let data = EventData::ServiceStart { id: "id".to_owned() };
        assert_eq!(data.description(), "id");
    }

    // TODO Factorize this test with the one above once there's a way to loop over a random emum.
    // https://github.com/rust-lang/rfcs/issues/284
    it "ServiceStop should return its ID as a description" {
        let data = EventData::ServiceStop { id: "id".to_owned() };
        assert_eq!(data.description(), "id");
    }
}
