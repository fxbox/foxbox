/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */
use openssl::ssl::{ Ssl, SslContext, SslMethod, SSL_VERIFY_NONE };
use openssl::ssl::error::SslError;
use openssl::x509::X509FileType;
use openssl_sys;

use std::collections::HashMap;
use std::io::Error;
use std::path::Path;
use std::sync::{ Arc, RwLock };

use certificate_record::CertificateRecord;

pub trait SslContextProvider : Send + Sync {
    fn context(&self) -> Result<SslContext, Error>;
    fn update(&self, HashMap<String, CertificateRecord>) -> ();
}

#[derive(Clone)]
pub struct SniSslContextProvider {
    main_context: Arc<RwLock<SslContext>>
}

impl SslContextProvider for SniSslContextProvider {
    fn context(&self) -> Result<SslContext, Error> {
        Ok(checklock!(self.main_context.read()).clone())
    }

    fn update(&self, configured_hosts: HashMap<String, CertificateRecord>) -> () {
        debug!("Updating SniSslContextProvider");

        let mut new_ssl_hosts = HashMap::new();

        for record in configured_hosts.values() {
            debug!("Creating SslContext for {}", record.hostname);
            let ssl_context = create_ssl_context(
                &record.cert_file, &record.private_key_file, &record.full_chain);

            if ssl_context.is_ok() {
                let ssl_context = ssl_context.unwrap();
                new_ssl_hosts.insert(record.hostname.clone(), ssl_context);
            } else {
                error!("Failed to configure SslContext for: {:?}", record);
            }
        }

        debug!("Update certificates");
        checklock!(self.main_context.write())
            .set_servername_callback_with_data(SniSslContextProvider::servername_callback, new_ssl_hosts);
    }
}

impl SniSslContextProvider {
    pub fn new() -> Self {
        SniSslContextProvider {
            main_context: Arc::new(
                              RwLock::new(
                                  SslContext::new(SslMethod::Sslv23).unwrap()
                              )
                          )
        }
    }

    fn servername_callback_impl<T>(ssl: &mut SslForSni<T>,
                                   configured_certs: &HashMap<String, T>) -> i32 {
        debug!("servername_callback invoked");
        let requested_hostname = ssl.get_hostname();

        if requested_hostname.is_none() {
            error!("No SNI information sent from client - client unsupported");
            return openssl_sys::SSL_TLSEXT_ERR_NOACK;
        }

        let requested_hostname = requested_hostname.unwrap();

        debug!("Selecting certificate for host {}", requested_hostname);

        let ssl_context_for_hostname = configured_certs.get(&requested_hostname);

        if let Some(ctx)= ssl_context_for_hostname {
            ssl.set_context(ctx);
        } else {
            error!("No certificate available for requested hostname: {}", requested_hostname);
        }

        openssl_sys::SSL_TLSEXT_ERR_OK
    }

    fn servername_callback(ssl: &mut Ssl, _: &mut i32,
                           configured_certs: &HashMap<String, SslContext>)
        -> i32 {
        Self::servername_callback_impl(ssl, configured_certs)
    }
}

impl Default for SniSslContextProvider {
    fn default() -> Self {
        SniSslContextProvider::new()
    }
}

pub trait SslForSni<T> {
    fn get_hostname(&self) -> Option<String>;
    fn set_context(&self, ctx: &T) -> Option<T>;
}

impl SslForSni<SslContext> for Ssl {
    fn get_hostname(&self) -> Option<String> {
        self.get_servername()
    }

    fn set_context(&self, ctx: &SslContext) -> Option<SslContext> {
        Some(self.set_ssl_context(ctx))
    }
}

pub fn create_ssl_context<C, K>(crt: &C, key: &K, chain: &Option<K>)
        -> Result<SslContext, SslError>
    where C: AsRef<Path>, K: AsRef<Path> {

    debug!("Creating SSL Context with Cert: {:?}, Key: {:?}",
            crt.as_ref().to_str(), key.as_ref().to_str());

    let mut ctx = try!(SslContext::new(SslMethod::Sslv23));
    try!(ctx.set_cipher_list("DEFAULT"));
    try!(ctx.set_certificate_file(crt.as_ref(), X509FileType::PEM));
    try!(ctx.set_private_key_file(key.as_ref(), X509FileType::PEM));

    if let Some(ref chain_file) = *chain {
        try!(ctx.set_certificate_chain_file(chain_file.as_ref(), X509FileType::PEM));
    }

    ctx.set_verify(SSL_VERIFY_NONE, None);

    Ok(ctx)
}

#[cfg(test)]
mod sni_ssl_context_provider {
    use openssl_sys;
    use std::collections::HashMap;
    use std::sync::mpsc::{ channel, Sender };

    use super::*;

    pub struct MockSsl {
        servername: Option<String>,
        context_set: Option<Sender<String>>
    }

    impl SslForSni<String> for MockSsl {
        fn get_hostname(&self) -> Option<String> {
            self.servername.clone()
        }

        fn set_context(&self, ctx: &String) -> Option<String> {
            if let Some(ref context_set) = self.context_set {
                context_set.send(ctx.clone()).unwrap();
            }

            None
        }
    }

    #[test]
    fn should_set_context_based_on_servername() {
        let (tx_context_called, rx_context_called) = channel();

        let mut ssl = MockSsl {
            servername: Some("test.knilxof.org".to_owned()),
            context_set: Some(tx_context_called)
        };

        let mut contexts = HashMap::new();

        contexts.insert("test.knilxof.org".to_owned(), "fake_context".to_owned());

        let result = SniSslContextProvider::servername_callback_impl(&mut ssl, &contexts);

        assert!(
            result == openssl_sys::SSL_TLSEXT_ERR_OK,
            "Servername callback did not return OK"
        );
        assert!(
            rx_context_called.recv().unwrap() == "fake_context",
            "Set context was not called with the expected value"
        );
    }

    #[test]
    fn should_return_fail_code_if_servername_is_not_available() {
        let mut ssl = MockSsl {
            servername: None,
            context_set: None,
        };

        let contexts = HashMap::new();
        let result = SniSslContextProvider::servername_callback_impl(&mut ssl, &contexts);

        assert!(
            result == openssl_sys::SSL_TLSEXT_ERR_NOACK,
            "Expected ERR_NOACK result from servername callback"
        );
    }
}
