// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use hyper::client::{Body, Client};
use hyper::net::{HttpsConnector, Openssl};
use hyper::status::StatusCode;
use openssl::ssl::error::SslError;
use serde_json;
use std::collections::BTreeMap;
use std::io;
use certificate_record::CertificateRecord;

const DNS_API_VERSION: &'static str = "v1";

pub struct DnsRecord<'a> {
    pub record_type: &'a str,
    pub name: &'a str,
    pub value: &'a str,
}

fn create_https_client(client: CertificateRecord) -> Result<Client, SslError> {

    let ssl_ctx = try!(Openssl::with_cert_and_key(&client.cert_file, &client.private_key_file));

    Ok(Client::with_connector(HttpsConnector::new(ssl_ctx)))
}

pub fn register_dns_record(client: CertificateRecord,
                           dns_record: &DnsRecord,
                           api_endpoint: &str)
                           -> io::Result<()> {

    if let Ok(https_client) = create_https_client(client) {

        let request_url =
            format!("{}/{}/dns{}",
                    api_endpoint,
                    DNS_API_VERSION,
                    dns_record.name.rsplit(".").fold("".to_owned(), |url, component| {
                        format!("{}/{}", url, component)
                    }));

        let mut map = BTreeMap::new();
        map.insert("type".to_owned(), dns_record.record_type);
        map.insert("value".to_owned(), dns_record.value);

        let payload = serde_json::to_vec(&map).unwrap();

        let result = https_client.post(&request_url)
            .body(Body::BufBody(&payload[..], payload.len()))
            .send();

        match result {
            Ok(response) => {
                if response.status == StatusCode::Ok {
                    info!("DNS API response 200 OK");
                    Ok(())
                } else {
                    error!("Could not register a DNS entry for {}", dns_record.name);
                    Err(io::Error::new(io::ErrorKind::Other,
                                       format!("Failed to register DNS record: HTTP Response \
                                                returned not OK (Was response code: {})",
                                               response.status)))
                }
            }
            Err(e) => {
                error!("Could not register a DNS entry for {}", dns_record.name);
                Err(io::Error::new(io::ErrorKind::Other,
                                   format!("Failed to register DNS record: {}", e)))
            }
        }
    } else {
        error!("Could not register a DNS entry for {}", dns_record.name);
        Err(io::Error::new(io::ErrorKind::Other,
                           "Failed to create an HTTPS client to set up a DNS record"))
    }
}
