/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use hyper::client::{ Body, Client };
use hyper::net::{ HttpsConnector, Openssl };
use hyper::status::StatusCode;

use serde_json;

use std::collections::{ BTreeMap, HashMap };
use std::fs::{ self };
use std::io::{ Error, ErrorKind };
use std::path::PathBuf;
use std::sync::{ Arc, Mutex, RwLock };

use tls::certificate_record::CertificateRecord;
use tls::ssl_context::SslContextProvider;

#[derive(Clone)]
pub struct CertificateManager {
    ssl_hosts: Arc<RwLock<HashMap<String, CertificateRecord>>>,

    // Observer
    context_provider: Option<Arc<Mutex<Box<SslContextProvider>>>>
}

fn create_records_from_directory(path: &PathBuf) -> Result<HashMap<String, CertificateRecord>, Error> {
    let mut records = HashMap::new();

    if try!(fs::metadata(path)).is_dir() {
        for entry in try!(fs::read_dir(path)) {
            let entry = try!(entry);

            let hostname = entry.file_name().into_string().unwrap();
            info!("Using certificate for host {}", hostname);

            let mut host_path = path.clone();
            host_path.push(hostname.clone());

            let mut cert_path = host_path.clone();
            cert_path.push("crt.pem");

            let mut private_key_file = host_path.clone();
            private_key_file.push("private_key.pem");

            records.insert(hostname.clone(),
                           try!(CertificateRecord::new(hostname,
                                                  cert_path,
                                                  private_key_file)));
        }

        info!("Loaded certificates from directory: {:?}", path);

        Ok(records)
    } else {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "The configured SSL certificate directory is not recognised as a directory."
        ))
    }
}

impl CertificateManager {
    pub fn new() -> CertificateManager {
        CertificateManager {
            ssl_hosts: Arc::new(RwLock::new(HashMap::new())),
            context_provider: None,
        }
    }

    pub fn set_context_provider(&mut self, context_provider: Arc<Mutex<Box<SslContextProvider>>>) {
        if self.context_provider.is_none() {
           self.context_provider = Some(context_provider);
           self.notify_provider();
        } else {
            error!("SslContextProvider was set more than once in the CertificateManager");
        }
    }

    pub fn reload_from_directory(&mut self, directory: PathBuf) -> Result<(), Error> {
        let certificates =  try!(create_records_from_directory(&directory));
        {
            let mut current_hosts = checklock!(self.ssl_hosts.write());
            current_hosts.clear();
            current_hosts.extend(certificates);
        }

        self.notify_provider();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn add_certificate(&mut self, certificate_record: CertificateRecord) {
        {
            checklock!(self.ssl_hosts.write())
                .insert(certificate_record.hostname.clone(), certificate_record);
        }

        self.notify_provider();
    }

    #[allow(dead_code)]
    pub fn get_certificate(&self, hostname: String) -> Option<CertificateRecord> {
        let ssl_hosts = checklock!(self.ssl_hosts.read());
        let cert_record = ssl_hosts.get(&hostname);

        if let Some(record) = cert_record {
            Some(record.clone())
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn remove_certificate(&mut self, hostname: String) {
        {
            checklock!(self.ssl_hosts.write()).remove(&hostname);
        }

        self.notify_provider();
    }

    fn create_https_client_with_crt_for(&self, hostname: String) -> Option<(Client, CertificateRecord)> {
        if let Some(certificate_record) = self.get_certificate(hostname.clone()) {
            let ssl_ctx = Openssl::with_cert_and_key(
                                &certificate_record.cert_file,
                                &certificate_record.private_key_file);

            if let Ok(ssl_ctx) = ssl_ctx {
                Some((Client::with_connector(HttpsConnector::new(ssl_ctx)), certificate_record))
            } else {
                None
            }
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn register_for_dns_challenge(&mut self, hostname: String, dns_endpoint: String, ip_address: String) -> Result<bool, String> {
        if let Some((https_client, cert_record)) = self.create_https_client_with_crt_for(hostname.clone()) {

            let hash = cert_record.get_certificate_fingerprint();

            let request_url = format!("{}/v1/dns/{}/{}", dns_endpoint,  hostname.rsplit(".").fold("".to_owned(), |url, component| {
                format!("{}/{}", url, component)
            }), hash);

            let mut map = BTreeMap::new();
            map.insert("type".to_owned(), "A".to_owned());
            map.insert("value".to_owned(), ip_address.clone());

            let payload = serde_json::to_vec(&map).unwrap();

            info!("Registering ip '{}' at '{}'", &ip_address, &request_url);
            https_client.post(&request_url)
                        .body(Body::BufBody(payload.as_slice(), payload.len()))
                        .send()
                        .or_else(|error| {
                            // import Error trait for description
                            use std::error::Error;
                            Err(error.description().to_owned())
                        })
                        .map(|response| {
                            response.status == StatusCode::Ok
                        })
        } else  {
            error!("Could not register a DNS entry for {}", hostname);
            Ok(false)
        }
    }

    fn notify_provider(&mut self) {
        let ssl_hosts = checklock!(self.ssl_hosts.read()).clone();

        if let Some(ref mut context_provider) = self.context_provider {
            context_provider.lock().unwrap().update(ssl_hosts);
        }
    }
}


#[cfg(test)]
mod certificate_manager {
    use openssl::ssl::{ SslContext, SslMethod };
    use std::collections::HashMap;
    use std::io::{ Error, ErrorKind };
    use std::path::PathBuf;
    use std::sync::{ Arc, Mutex };
    use std::sync::mpsc::{ channel, Sender };
    use tls::{ CertificateRecord, SslContextProvider };

    use super::*;

    pub struct TestSslContextProvider {
        update_called: Sender<bool>
    }

    impl TestSslContextProvider {
        fn new(update_chan: Sender<bool>) -> Self {
            TestSslContextProvider {
                update_called: update_chan
            }
        }
    }

    impl SslContextProvider for TestSslContextProvider {
        fn context(&self) -> Result<SslContext, Error> {
            SslContext::new(SslMethod::Sslv23).map_err(|_| {
                Error::new(ErrorKind::InvalidInput, "An SSL certificate could not be configured")
            })
        }

        fn update(&mut self, _: HashMap<String, CertificateRecord>) -> () {
            self.update_called.send(true).unwrap();
        }
    }

    fn test_cert_record() -> CertificateRecord {
        CertificateRecord::new_for_test(
            "test.example.com".to_owned(),
            PathBuf::from("/test/key.pem"),
            PathBuf::from("/test/crt.pem"),
            "010203040506070809000a0b0c0d0e0f".to_owned()
        ).unwrap()
    }

    #[test]
    fn should_allow_certificates_to_be_added() {
        let cert_record = test_cert_record();
        let mut cert_manager = CertificateManager::new();

        cert_manager.add_certificate(cert_record.clone());

        assert!(cert_manager.get_certificate("test.example.com".to_owned()).unwrap() == cert_record);

        cert_manager.remove_certificate("test.example.com".to_owned());

        assert!(cert_manager.get_certificate("test.example.com".to_owned()).is_none());
    }

    #[test]
    fn should_allow_certificates_to_be_removed() {
        let cert_record = test_cert_record();
        let mut cert_manager = CertificateManager::new();

        cert_manager.add_certificate(cert_record);

        cert_manager.remove_certificate("test.example.com".to_owned());

        assert!(cert_manager.get_certificate("test.example.com".to_owned()).is_none());
    }

    #[test]
    fn should_update_configured_providers_when_cert_added() {
        let cert_record = test_cert_record();
        let (tx_update_called, rx_update_called) = channel();
        let mut cert_manager = CertificateManager::new();

        let provider_one = Box::new(TestSslContextProvider::new(tx_update_called));

        cert_manager.set_context_provider(Arc::new(Mutex::new(provider_one)));

        cert_manager.add_certificate(cert_record);

        assert!(rx_update_called.recv().unwrap(), "Did not receive notification from handler after add");
    }

    #[test]
    fn should_update_configured_providers_when_cert_removed() {
        let cert_record = test_cert_record();
        let (tx_update_called, rx_update_called) = channel();
        let mut cert_manager = CertificateManager::new();

        let provider_one = Box::new(TestSslContextProvider::new(tx_update_called));

        cert_manager.set_context_provider(Arc::new(Mutex::new(provider_one)));

        cert_manager.add_certificate(cert_record);

        assert!(rx_update_called.recv().unwrap(), "Did not receive notification from handler after add");

        cert_manager.remove_certificate(test_cert_record().hostname);

        assert!(rx_update_called.recv().unwrap(), "Did not receive notification from handler after remove");
    }
}
