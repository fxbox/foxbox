/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use service::Service;
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };

// The `global` context available to all.
pub struct Context {
    pub verbose: bool,
    pub services: HashMap<String, Box<Service>>
}

pub type SharedContext = Arc<Mutex<Context>>;

impl Context {
    pub fn new(verbose: bool) -> Context {
        Context { services: HashMap::new(),
                  verbose: verbose }
    }

    pub fn shared(verbose: bool) -> SharedContext {
        Arc::new(Mutex::new(Context::new(verbose)))
    }
}
