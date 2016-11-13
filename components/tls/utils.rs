// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
use openssl::crypto::hash::Type;
use openssl::crypto::pkey::PKey;
use openssl::ssl::error::SslError;
use openssl::x509::{X509, X509Generator};

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use certificate_record::CertificateRecord;

/// Create a `HashMap` of `CertificateRecord`s (hashed by the hostname it was created for)
pub fn create_records_from_directory(path: &PathBuf)
                                     -> Result<HashMap<String, CertificateRecord>, io::Error> {
    let mut records = HashMap::new();

    // Ensure the directory exists.
    fs::create_dir_all(path).unwrap_or_else(|err| {
        if err.kind() != io::ErrorKind::AlreadyExists {
            panic!("Unable to create directory: {:?}: {}", path, err);
        }
    });

    // Ensure the directory really is a directory.
    if try!(fs::metadata(path)).is_dir() {
        for entry in try!(fs::read_dir(path)) {
            let entry = try!(entry);

            // Won't support symlinks
            let file_type = try!(entry.file_type());
            if file_type.is_dir() || file_type.is_symlink() {

                let hostname = entry.file_name().into_string().unwrap();
                debug!("Loading certificate for host {}", hostname);

                let mut host_path = path.clone();
                host_path.push(hostname.clone());

                let mut cert_path = host_path.clone();
                cert_path.push("cert.pem");

                let mut private_key_file = host_path.clone();
                private_key_file.push("privkey.pem");

                let mut fullchain_file = host_path.clone();
                fullchain_file.push("fullchain.pem");

                let fullchain = if fullchain_file.as_path().exists() {
                    Some(fullchain_file)
                } else {
                    None
                };

                records.insert(hostname.clone(),
                               try!(CertificateRecord::new(hostname,
                                                           cert_path,
                                                           private_key_file,
                                                           fullchain)));
            }
        }

        info!("Loaded certificates from directory: {:?}", path);
        Ok(records)
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidInput,
                           "The configured SSL certificate directory is not recognised as a \
                            directory."))
    }
}

/// Generate a self signed certificate and store the resulting
/// private key and certificate PEM on disk - returning a
/// `CertificateRecord` if the generation succeeded.
pub fn generate_self_signed_certificate<P>(hostname: &str,
                                           certificate_directory: P)
                                           -> io::Result<CertificateRecord>
    where P: AsRef<Path>
{

    let mut directory = certificate_directory.as_ref().to_path_buf();
    directory.push(hostname);

    info!("Generating new self-signed cert for {}", hostname);
    let generator = X509Generator::new()
        .set_bitlength(2048)
        .set_valid_period(365 * 2)
        .add_name("CN".to_owned(), String::from(hostname))
        .set_sign_hash(Type::SHA256);

    let gen_result = generator.generate();
    if let Ok((cert, pkey)) = gen_result {

        // Write the cert and pkey PEM files

        // Ensure the directory exists
        fs::create_dir_all(directory.clone()).unwrap_or_else(|err| {
            if err.kind() != io::ErrorKind::AlreadyExists {
                panic!("Unable to create directory {:?}: {}", directory, err);
            }
        });

        let mut cert_path = directory.clone();
        cert_path.push("cert.pem");

        let mut pkey_path = directory.clone();
        pkey_path.push("privkey.pem");

        let write_result = write_pem(&pkey, &pkey_path);
        if write_result.is_err() {
            return Err(write_result.unwrap_err());
        }

        let write_result = write_pem(&cert, &cert_path);
        if write_result.is_err() {
            return Err(write_result.unwrap_err());
        }

        CertificateRecord::new(String::from(hostname), cert_path, pkey_path, None)
    } else {
        let e = gen_result.err().unwrap();
        Err(io::Error::new(io::ErrorKind::InvalidData,
                           format!("Failed to generate self signed certificate: {}",
                                   e.description())))
    }
}

#[cfg(test)]
mod tests {
    use mktemp::Temp;
    use super::*;
    #[test]
    fn test_generate_self_signed_cert() {
        let temp_dir = Temp::new_dir().unwrap();

        // If the files don't exist, the certificate record
        // creation will have failed, unwrap() will panic
        // if anything failed.
        let _ = generate_self_signed_certificate("atestdomain.knilxof.org", temp_dir).unwrap();
    }
}

fn write_pem<TWriter, P>(pem_writer: &TWriter, path: P) -> io::Result<()>
    where TWriter: PemWriter,
          P: AsRef<Path>
{

    let file_create_result = fs::File::create(path.as_ref());
    if let Ok(mut file_handle) = file_create_result {
        pem_writer.write(&mut file_handle).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData,
                           format!("Failed to write PEM {:?}: {}",
                                   path.as_ref(),
                                   e.description()))
        })
    } else {
        Err(file_create_result.unwrap_err())
    }
}

trait PemWriter {
    fn write<W: io::Write>(&self, writer: &mut W) -> Result<(), SslError>;
}

impl<'a> PemWriter for X509<'a> {
    fn write<W: io::Write>(&self, writer: &mut W) -> Result<(), SslError> {
        self.write_pem(writer)
    }
}

impl PemWriter for PKey {
    fn write<W: io::Write>(&self, writer: &mut W) -> Result<(), SslError> {
        self.write_pem(writer)
    }
}
