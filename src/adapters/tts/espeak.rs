// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate libc;

use adapters::tts::engine::TtsEngine;
use libc::{c_int, c_char, c_void, size_t, c_uint};

/// Basic espeak bindings.

pub const ESPEAK_CHARS_UTF8: c_uint = 1;

#[repr(C)]
#[allow(dead_code)]
pub enum espeak_POSITION_TYPE {
    POS_CHARACTER = 1,
    POS_WORD,
    POS_SENTENCE,
}

#[repr(C)]
#[allow(dead_code)]
pub enum espeak_AUDIO_OUTPUT {
    AUDIO_OUTPUT_PLAYBACK,
    AUDIO_OUTPUT_RETRIEVAL,
    AUDIO_OUTPUT_SYNCHRONOUS,
    AUDIO_OUTPUT_SYNCH_PLAYBACK,
}

#[repr(C)]
#[allow(dead_code)]
pub enum espeak_ERROR {
    EE_OK = 0,
    EE_INTERNAL_ERROR = -1,
    EE_BUFFER_FULL = 1,
    EE_NOT_FOUND = 2,
}

#[link(name = "espeak")]
#[allow(dead_code)]
extern "C" {
    pub fn espeak_Initialize(output: espeak_AUDIO_OUTPUT,
                             buflength: c_int,
                             path: *const c_char,
                             options: c_int)
                             -> c_int;
    pub fn espeak_Synth(text: *const c_void,
                        size: size_t,
                        position: c_uint,
                        position_type: espeak_POSITION_TYPE,
                        end_position: c_uint,
                        flags: c_uint,
                        unique_identifier: *mut c_uint,
                        user_data: *mut c_void)
                        -> espeak_ERROR;
    pub fn espeak_Terminate() -> espeak_ERROR;
}

pub struct EspeakEngine;

impl TtsEngine for EspeakEngine {
    fn init(&self) -> bool {
        use std::ptr;

        let res;
        unsafe {
            res = espeak_Initialize(espeak_AUDIO_OUTPUT::AUDIO_OUTPUT_PLAYBACK,
                                    0, // Buffer length. 0 == 200ms
                                    ptr::null(), // eSpeak-data dir
                                    0 /* Options. */);
        }
        res != -1
    }

    fn say(&self, text: &str) {
        use std::ffi::CString;
        use std::ptr;
        use std::thread;

        let text = String::from(text);
        let len = text.len();
        let s = CString::new(text.clone()).unwrap();

        thread::spawn(move || {
            unsafe {
                espeak_Synth(s.as_ptr() as *const libc::c_void, // Sentence to speak.
                             len + 1, // Size in bytes of the sentence. Not used in synchronous mode.
                             0, // Start position.
                             espeak_POSITION_TYPE::POS_CHARACTER, // Position type.
                             0, // End position.
                             ESPEAK_CHARS_UTF8, // Flags.
                             ptr::null_mut(), // Unique id.
                             ptr::null_mut() /* Opaque user data. */);
            }
        });
    }

    fn shutdown(&self) {
        unsafe {
            espeak_Terminate();
        }
    }
}
