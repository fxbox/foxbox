extern crate libc;
extern crate crypto;

use self::crypto::aead::AeadEncryptor;
use self::crypto::aes_gcm::AesGcm;
use self::crypto::aes::KeySize;
use self::crypto::hkdf::hkdf_extract;
use self::crypto::hmac::Hmac;
use self::crypto::sha2::Sha256;
use self::crypto::mac::{ Mac, MacResult };

use std::ffi::{ CString, CStr };
use std::ptr;
use rand::Rng;
use rand::os::OsRng;

use rustc_serialize::base64::{ FromBase64, ToBase64, URL_SAFE };
use rustc_serialize::hex::{ ToHex, FromHex };

const NID_X9_62_PRIMVE256V1: libc::c_int = 415;

const EVP_PKEY_EC: libc::c_int = 408;
const EVP_PKEY_OP_PARAMGEN: libc::c_int = 2;
const EVP_PKEY_CTRL_EC_PARAMGEN_CURVE_NID: libc::c_int = 4097;

#[repr(C)]
enum EcPointConversion {
    Uncompressed = 4
}

type EcKey = libc::c_void;
type EcPoint = libc::c_void;
type EcGroup = libc::c_void;
type EvpPkey = libc::c_void;
type EvpPkeyCtx = libc::c_void;
type BnCtx = libc::c_void;

struct WrappedEcKey {
    p: *mut EcKey
}

impl WrappedEcKey {
    fn new(p: *mut EcKey) -> Self {
        WrappedEcKey { p: p }
    }
}

/*
impl Drop for WrappedEcKey {
    fn drop(&mut self) {
        if !self.p.is_null() {
            unsafe { EC_KEY_free(self.p); }
        }
    }
}
*/

struct WrappedEcPoint {
    p: *mut EcPoint
}

impl WrappedEcPoint {
    fn new(p: *mut EcPoint) -> Self {
        WrappedEcPoint { p: p }
    }
}

/*
impl Drop for WrappedEcPoint {
    fn drop(&mut self) {
        if !self.p.is_null() {
            unsafe { EC_POINT_free(self.p); }
        }
    }
}
*/

struct WrappedEvpPkey {
    p: *mut EvpPkey
}

impl WrappedEvpPkey {
    fn new(p: *mut EvpPkey) -> Self {
        WrappedEvpPkey { p: p }
    }

    fn null() -> Self {
        WrappedEvpPkey { p: ptr::null_mut() }
    }
}

/*
impl Drop for WrappedEvpPkey {
    fn drop(&mut self) {
        if !self.p.is_null() {
            unsafe { EVP_PKEY_free(self.p); }
        }
    }
}
*/

struct WrappedEvpPkeyCtx {
    p: *mut EvpPkeyCtx
}

impl WrappedEvpPkeyCtx {
    fn new(p: *mut EvpPkeyCtx) -> Self {
        WrappedEvpPkeyCtx { p: p }
    }
}

/*
impl Drop for WrappedEvpPkeyCtx {
    fn drop(&mut self) {
        if !self.p.is_null() {
            unsafe { EVP_PKEY_CTX_free(self.p); }
        }
    }
}
*/

extern "C" {
    fn EC_KEY_new_by_curve_name(nid: libc::c_int) -> *mut EcKey;
    fn EC_KEY_free(key: *mut EcKey);
    fn EC_KEY_get0_group(key: *const EcKey) -> *const EcGroup;
    fn EC_KEY_set_public_key(key: *mut EcKey, pub_key: *const EcPoint) -> libc::c_int;
    fn EC_KEY_get0_public_key(key: *const EcKey) -> *mut EcPoint;

    fn EC_POINT_hex2point(group: *const EcGroup, hex: *const libc::c_char, p: *mut EcPoint, ctx: *mut BnCtx) -> *mut EcPoint;
    fn EC_POINT_point2hex(group: *const EcGroup, point: *const EcPoint, form: EcPointConversion, ctx: *mut BnCtx) -> *mut libc::c_char;
    //fn EC_POINT_point2buf(group: *const EcGroup, point: *const EcPoint, form: EcPointConversion, pbuf: *mut *mut libc::c_char, ctx: *mut BnCtx) -> libc::size_t;
    fn EC_POINT_free(point: *mut EcPoint);

    fn EVP_PKEY_new() -> *mut EvpPkey;
    fn EVP_PKEY_free(pkey: *mut EvpPkey);
    fn EVP_PKEY_set1_EC_KEY(evpKey: *mut EvpPkey, ecKey: *mut EcKey) -> libc::c_int;
    fn EVP_PKEY_get1_EC_KEY(pkey: *mut EvpPkey) -> *mut EcKey;

    fn EVP_PKEY_CTX_new_id(id: libc::c_int, engine: *mut libc::c_void) -> *mut EvpPkeyCtx;
    fn EVP_PKEY_CTX_new(pkey: *mut EvpPkey, engine: *mut libc::c_void) -> *mut EvpPkeyCtx;
    fn EVP_PKEY_CTX_free(ctx: *mut EvpPkeyCtx);
    fn EVP_PKEY_CTX_ctrl(ctx: *mut EvpPkeyCtx, keytype: libc::c_int, optype: libc::c_int, cmd: libc::c_int, p1: libc::c_int, p2: *mut libc::c_void) -> libc::c_int;

    fn EVP_PKEY_paramgen_init(ctx: *mut EvpPkeyCtx) -> libc::c_int;
    fn EVP_PKEY_paramgen(ctx: *mut EvpPkeyCtx, ppkey: *mut *mut EvpPkey) -> libc::c_int;

    fn EVP_PKEY_keygen_init(ctx: *mut EvpPkeyCtx) -> libc::c_int;
    fn EVP_PKEY_keygen(ctx: *mut EvpPkeyCtx, ppkey: *mut *mut EvpPkey) -> libc::c_int;

    fn EVP_PKEY_derive_init(ctx: *mut EvpPkeyCtx) -> libc::c_int;
    fn EVP_PKEY_derive_set_peer(ctx: *mut EvpPkeyCtx, peer: *mut EvpPkey) -> libc::c_int;
    fn EVP_PKEY_derive(ctx: *mut EvpPkeyCtx, key: *mut libc::c_char, size: *mut libc::size_t) -> libc::c_int;
}

#[derive(Debug)]
struct EcdhKeyData {
    public_key: String,
    shared_key: Vec<u8>
}

fn ecdh_derive_keys(raw_peer_key : String) -> Result<EcdhKeyData, String> {
    let native_peer_key = CString::new(raw_peer_key).unwrap();

    unsafe {
        /* First we need to derive the OpenSSL pkey representation of the peer's public key */
        let eckey = WrappedEcKey::new(EC_KEY_new_by_curve_name(NID_X9_62_PRIMVE256V1));
        if eckey.p.is_null() {
            return Err(String::from("Cannot create EC X9.62 key"));
        }

        let ecpt = WrappedEcPoint::new(EC_POINT_hex2point(EC_KEY_get0_group(eckey.p), native_peer_key.as_ptr(), ptr::null_mut(), ptr::null_mut()));
        if ecpt.p.is_null() {
            return Err(String::from("Cannot convert raw EC public key to EC point"));
        }

        if EC_KEY_set_public_key(eckey.p, ecpt.p) != 1 {
            return Err(String::from("Cannot set EC public key"));
        }

        let peer_key = WrappedEvpPkey::new(EVP_PKEY_new());
        if peer_key.p.is_null() {
            return Err(String::from("Cannot create peer key"));
        }

        if EVP_PKEY_set1_EC_KEY(peer_key.p, eckey.p) != 1 {
            return Err(String::from("Cannot initialize peer key from EC key"));
        }

        /* Second we need to construct a context to generate our own private/public key pair */
        let param_ctx = WrappedEvpPkeyCtx::new(EVP_PKEY_CTX_new_id(EVP_PKEY_EC, ptr::null_mut()));
        if param_ctx.p.is_null() {
            return Err(String::from("Cannot create param context"));
        }

        if EVP_PKEY_paramgen_init(param_ctx.p) != 1 {
            return Err(String::from("Cannot init param context"));
        }

        if EVP_PKEY_CTX_ctrl(param_ctx.p, EVP_PKEY_EC, EVP_PKEY_OP_PARAMGEN, EVP_PKEY_CTRL_EC_PARAMGEN_CURVE_NID, NID_X9_62_PRIMVE256V1, ptr::null_mut()) != 1 {
            return Err(String::from("Cannot set param context as X9.62 P256V1"));
        }

        let mut params = WrappedEvpPkey::null();
        if EVP_PKEY_paramgen(param_ctx.p, &mut params.p) != 1 || params.p.is_null() {
            return Err(String::from("Cannot generate params from context"));
        }

        let key_ctx = WrappedEvpPkeyCtx::new(EVP_PKEY_CTX_new(params.p, ptr::null_mut()));
        if key_ctx.p.is_null() {
            return Err(String::from("Cannot create key context"));
        }

        if EVP_PKEY_keygen_init(key_ctx.p) != 1 {
            return Err(String::from("Cannot init key context"));
        }

        let mut local_key = WrappedEvpPkey::null();
        if EVP_PKEY_keygen(key_ctx.p, &mut local_key.p) != 1 || local_key.p.is_null() {
            return Err(String::from("Cannot generate public/private key pair from key context"));
        }

        /* Third we need to derive a shared key from our private/public key pair and the peer's
         * public key */
        let shared_ctx = WrappedEvpPkeyCtx::new(EVP_PKEY_CTX_new(local_key.p, ptr::null_mut()));
        if shared_ctx.p.is_null() {
            return Err(String::from("Cannot create shared key context"));
        }

        if EVP_PKEY_derive_init(shared_ctx.p) != 1 {
            return Err(String::from("Cannot init shared context"));
        }

        if EVP_PKEY_derive_set_peer(shared_ctx.p, peer_key.p) != 1 {
            return Err(String::from("Cannot set peer key for shared context"));
        }

        let mut shared_len : libc::size_t = 0;
        if EVP_PKEY_derive(shared_ctx.p, ptr::null_mut(), &mut shared_len) != 1 {
            return Err(String::from("Cannot get shared key length from shared context"));
        }

        let shared_key : Vec<u8> = vec![0; shared_len];
        if EVP_PKEY_derive(shared_ctx.p, shared_key.as_ptr() as *mut libc::c_char, &mut shared_len) != 1 {
            return Err(String::from("Cannot get shared key from shared context"));
        }

        /* Fourth we need to get our public key to send to the remote end */
        let local_eckey = WrappedEcKey::new(EVP_PKEY_get1_EC_KEY(local_key.p));
        if local_eckey.p.is_null() {
            return Err(String::from("Cannot get local ec key from local key"));
        }

        let local_ecpoint = WrappedEcPoint::new(EC_KEY_get0_public_key(local_eckey.p));
        if local_ecpoint.p.is_null() {
            return Err(String::from("Cannot get public key from local ec key"));
        }

        let local_pkbuf = EC_POINT_point2hex(EC_KEY_get0_group(local_eckey.p), local_ecpoint.p, EcPointConversion::Uncompressed, ptr::null_mut());
        if local_pkbuf == ptr::null_mut() {
            return Err(String::from("Cannot get uncompressed public key from local ec point"));
        }

        /* API missing on older platforms
        let mut local_pkbuf : *mut libc::c_char = ptr::null_mut();
        let local_pkbuf_size = EC_POINT_point2buf(EC_KEY_get0_group(local_eckey.p), local_ecpoint.p, EcPointConversion::Uncompressed, &mut local_pkbuf, ptr::null_mut());
        if local_pkbuf == ptr::null_mut() || local_pkbuf_size == 0 {
            return Err(String::from("Cannot get uncompressed public key from local ec point"));
        }*/

        // FIXME: free local_pkbuf
        Ok(EcdhKeyData {
            public_key: CStr::from_ptr(local_pkbuf).to_string_lossy().into_owned(),
            shared_key: shared_key
        })
    }
}

#[derive(Debug)]
pub struct EncryptData {
    pub public_key: String,
    pub salt: String,
    pub output: Vec<u8>
}

pub fn encrypt(peer_key: &String, data: &String) -> Option<EncryptData> {
    let peer_key_bytes = match peer_key.from_base64() {
        Ok(x) => x,
        Err(e) => {
            warn!("Could not base64 decode peer key: {:?}", e);
            return None;
        }
    };

    // Derive public and secret keys from peer public key
    let ecdh = match ecdh_derive_keys(peer_key_bytes.to_hex()) {
        Ok(ekd) => ekd,
        Err(e) => {
            warn!("Could not derive keys: {}", e);
            return None;
        }
    };

    // Create the salt
    let mut gen = OsRng::new().unwrap();
    let mut salt = vec![0u8; 16];
    gen.fill_bytes(salt.as_mut_slice());

    // Create the HKDF
    let mut salt_hmac = Hmac::new(Sha256::new(), salt.as_slice());
    salt_hmac.input(ecdh.shared_key.as_slice());
    let salt_hmac_res = salt_hmac.result();

    // Create the encryiption key and nonce
    let mut encrypt_info = String::from("Content-Encoding: aesgcm128").into_bytes();
    // Add padding in accordance with
    // https://tools.ietf.org/html/draft-ietf-httpbis-encryption-encoding-00#section-3.3
    //encrypt_info.push(0);
    encrypt_info.push(1);
    let mut encrypt_key = vec![0u8; 32];
    hkdf_extract(Sha256::new(), salt_hmac_res.code(), encrypt_info.as_slice(), encrypt_key.as_mut_slice());
    encrypt_key.truncate(16);

    // Create the nonce
    let mut nonce_info = String::from("Content-Encoding: nonce").into_bytes();
    //nonce_info.push(0);
    nonce_info.push(1);
    let mut nonce = vec![0u8; 32];
    hkdf_extract(Sha256::new(), salt_hmac_res.code(), nonce_info.as_slice(), nonce.as_mut_slice());
    nonce.truncate(12);

    // Encrypt the payload with derived key and nonce
    let mut raw_data = data.clone().into_bytes();
    // Add padding in accordance with
    // https://tools.ietf.org/html/draft-ietf-httpbis-encryption-encoding-00#section-2
    raw_data.insert(0, 0);
    let mut cipher = AesGcm::new(KeySize::KeySize128, encrypt_key.as_slice(), nonce.as_slice(), &[0; 0]);
    let mut out: Vec<u8> = vec![0; raw_data.len()];
    let mut out_tag: Vec<u8> = vec![0; 16];
    cipher.encrypt(raw_data.as_slice(), &mut out, &mut out_tag);
    out.extend_from_slice(out_tag.as_slice());

    let public_key_bytes = match ecdh.public_key.from_hex() {
        Ok(x) => x,
        Err(e) => {
            warn!("Could not derive keys: {:?}", e);
            return None;
        }
    };

    let ed = EncryptData {
        public_key: public_key_bytes.to_base64(URL_SAFE),
        salt: salt.to_base64(URL_SAFE),
        output: out
    };

    info!("ed: {:?}", ed);
    Some(ed)
}

