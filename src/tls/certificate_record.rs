/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::path::PathBuf;

/// Defines a certificate, including the hostname it is for,
/// the private key file and the certificate file.
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct CertificateRecord {
    pub hostname: String,
    pub private_key_file: PathBuf,
    pub cert_file: PathBuf,
    // TODO: We should probably keep a hash of the
    // file contents as part of the CertRecord.
    // See: https://github.com/fxbox/foxbox/issues/224
}
