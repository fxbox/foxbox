// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::env;
use std::fs;
use std::path::Path;
extern crate pkg_config;

fn update_local_git_hook() {
    let p = env::current_dir().unwrap();
    let origin_path = Path::new(&p).join("./tools/pre-commit");
    let dest_path = Path::new(&p).join(".git/hooks/pre-commit");

    fs::copy(&origin_path, &dest_path).unwrap();
}

fn cp_r(origin: &Path, dest: &Path) {
    let dir = fs::read_dir(origin).unwrap();
    for file in dir {
        let file = file.unwrap();
        let origin_buf = origin.join(file.file_name());
        let origin = origin_buf.as_path();
        let dest_buf = dest.join(file.file_name());
        let dest = dest_buf.as_path();

        if file.file_type().unwrap().is_dir() {
            let dest_str = dest.to_str().unwrap();
            if let Err(_) = fs::metadata(&dest_str) {
                fs::create_dir(dest_str).unwrap();
            }
            cp_r(origin, dest);
        } else {
            fs::copy(origin, dest).unwrap();
        }
    }
}

fn copy_shared_static_files() {
    let current = env::current_dir().unwrap();
    let shared = Path::new(&current).join("./static/shared");
    for dest in vec!["./static/setup/shared", "./static/main/shared"] {
        let dest = Path::new(&current).join(dest);
        let dest_str = dest.to_str().unwrap();
        if let Err(_) = fs::metadata(&dest_str) {
            fs::create_dir(dest_str).unwrap();
        }
        cp_r(&shared, &dest);
    }
}

fn link_external_libs() {
    pkg_config::probe_library("libupnp").unwrap();
}

fn main() {
    update_local_git_hook();
    link_external_libs();
    copy_shared_static_files();
}
