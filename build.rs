/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::env;
use std::fs;
use std::path::{ Path, PathBuf };
use std::process::Command;
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

// Looks for an environmental variable, defaulting to the empty string.
fn get_env(key: &str) -> String {
    env::var(key).unwrap_or("".to_owned())
}

// Returns the appropriate --target=XXX command line argument, or None
// if we are not cross compiling.
fn get_target() -> Option<String> {
    let target = get_env("TARGET");
    let host = get_env("HOST");
    if host != target {
        return Some(format!("--target={}", target));
    }

    None
}

fn cargo_build_in_directory(directory: &str) {
    let current_dir = env::current_dir().unwrap();
    let mut run_in_dir = PathBuf::from(&current_dir);
    run_in_dir.push(directory);

    let mut exec = Command::new("cargo");
    let mut command = exec.arg("build")
                          .arg("--verbose"); // Ideally should reflect the --verbose flag of the main
                                             // cargo command.

    let target = get_target();
    if target.is_some() {
        let tmp = command;
        command = tmp.arg(target.unwrap());
    }

    // $PROFILE is either `debug` (default value) or `release`.
    if get_env("PROFILE") == "release" {
        let tmp = command;
        command = tmp.arg("--release");
    }

    command.current_dir(run_in_dir)
           .spawn()
           .unwrap()
           .wait()
           .unwrap();

    /*if !output.status.success() {
        println!("status: {}", output.status);
        println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
        println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        panic!("debug"); // Forces the println!(...) to display.
    }*/
}

fn link_external_libs() {
    pkg_config::probe_library("libupnp").unwrap();
}

fn main() {
    update_local_git_hook();
    link_external_libs();
    copy_shared_static_files();
    cargo_build_in_directory("./components/dns_challenge");
}
