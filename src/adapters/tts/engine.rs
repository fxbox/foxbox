// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/// Simple trait to abstract the TTS engine implementation.
pub trait TtsEngine: Send + Sync {
    fn init(&self) -> bool;
    fn shutdown(&self);
    fn say(&self, text: &str);
}
