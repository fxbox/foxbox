/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate crypto;

use self::crypto::digest::Digest;
use self::crypto::sha1::Sha1;
use uuid::Uuid;

#[allow(dead_code)]
pub fn from_str(seed: String) -> Uuid {
    let mut sha = Sha1::new();
    sha.input_str(&seed);
    let mut buffer = [0u8; 20];
    sha.result(&mut buffer);
    Uuid::from_bytes(&buffer[..16]).unwrap()
}

#[cfg(test)]
describe! stable_uuid {
    it "should generate stable UUIDs" {
        let first_uuid = from_str("foofoo".to_owned());
        let second_uuid = from_str("foofoo".to_owned());
        assert_eq!(first_uuid, second_uuid);
    }
}
