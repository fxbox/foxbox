/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
use mktemp::Temp;
use std::fs::File;
use std::io;
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{ Path, PathBuf };
use std::process::Command;
use std::sync::mpsc::{ channel, Receiver };
use std::thread;
use CertificateManager;

const LETS_ENCRYPT_CLIENT: &'static str = include_str!("scripts/letsencrypt.sh");

/// Get a SAN certificate from `LetsEncrypt` for a given list of names.
pub fn get_san_cert_for<T>(names: T, certificate_manager: CertificateManager,
                           dns_endpoint: String)
        -> Receiver<io::Result<()>>
    where T: Iterator<Item=String>,
          T: DoubleEndedIterator,
          T: Clone + Send + 'static {

    let (tx, rx) = channel();

    thread::spawn(move || {
        tx.send(
            _get_san_cert_for(
                names, certificate_manager,
                &dns_endpoint)
          ).unwrap();
    });

    rx
}

/// Blocking version of `get_san_cert_for`
fn _get_san_cert_for<T>(names: T, certificate_manager: CertificateManager, dns_endpoint: &str)
    -> io::Result<()>
    where T: Iterator<Item=String>,
          T: DoubleEndedIterator,
          T: Clone + 'static {
    let temp_dir = try!(Temp::new_dir());

    let mut letsencrypt_file = temp_dir.to_path_buf();
    {
        letsencrypt_file.push("letsencrypt.sh");

        debug!("Creating letsencrypt client");
        try!(File::create(letsencrypt_file.clone()).and_then(|mut f| {
            f.write_all(LETS_ENCRYPT_CLIENT.as_bytes())
        }));
    }

    let mut domains_file = temp_dir.to_path_buf();
    {
        domains_file.push("domains.txt");

        let domains_txt = names.clone().rev().fold("".to_owned(), |accumulator, name| {
            format!("{} {}", name, accumulator)
        });

        debug!("Creating domains.txt for letsencrypt client");
        try!(File::create(domains_file).and_then(|mut f| {
            f.write_all(domains_txt.as_bytes())
        }));
    }

    let mut dns_challenge_file = temp_dir.to_path_buf();
    {
        dns_challenge_file.push("dns-challenge.sh");

        try!(File::create(dns_challenge_file.clone()).and_then(|mut f| {
            f.write_all(
                create_challenge_script(
                    &certificate_manager.get_certs_dir(),
                    &dns_endpoint
                ).as_bytes()
            )
        }));

        assert!(dns_challenge_file.as_path().exists());
    }

    let command = format!(
        "unset DYLD_LIBRARY_PATH && chmod +x {} && bash {} --cron --challenge dns-01 --hook {} && cp -R {}/certs/* {}",
        dns_challenge_file.to_str().unwrap(),
        letsencrypt_file.to_str().unwrap(),
        dns_challenge_file.to_str().unwrap(),
        temp_dir.to_path_buf().to_str().unwrap(),
        certificate_manager.get_certs_dir().to_str().unwrap());

    debug!("Spawning letsencrypt client {}", command);
    let mut child = try!(Command::new("/usr/bin/env")
                                 .arg("sh")
                                 .arg("-c")
                                 .arg(command)
                                 .spawn());

    let ecode = try!(child.wait());

    if !ecode.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "The LetsEncryt client failed - certificates could not be created",
        ))
    }

    let mut names = names;

    // SAN domain list (this is the Common Name (CN) of the cert)
    if let Some(common_name) = names.next() {
        let certs_dir = certificate_manager.get_certs_dir();

        for subject_alt_name in names {
            info!("Trying to link {:?} -> {:?}", subject_alt_name, common_name);

            let mut san_dir = certs_dir.clone();
            san_dir.push(subject_alt_name);
            try!(symlink(PathBuf::from(common_name.clone()), san_dir));
        }
    }

    Ok(())
}

fn create_challenge_script<T: AsRef<Path>>(cert_path: T, dns_endpoint: &str) -> String {
    format!("#!/usr/bin/env bash
echo $@
if [[ $1 == \"deploy_challenge\" ]];then
    RUST_BACKTRACE=1 CERTIFICATE_DIRECTORY={cert_path:?} DNS_API_ENDPOINT={dns_endpoint:?} dnschallenge $2 \"$4\"
fi
",

        cert_path=cert_path.as_ref(),
        dns_endpoint=dns_endpoint
    )
}
