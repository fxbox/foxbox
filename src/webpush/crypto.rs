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

    fn CRYPTO_free(ptr: *mut libc::c_void);
}

#[derive(Debug)]
struct EcdhKeyData {
    public_key: String,
    shared_key: Vec<u8>
}

/// Creates an OpenSSL representation of the given ECDH X9.62 public key,
/// represented as a string of hex digits.
fn ecdh_import_public_key(public_key : String) -> *mut EvpPkey {
    let eckey;
    let mut ecpt = ptr::null_mut();
    let mut peer_key = ptr::null_mut();
    let native_key = CString::new(public_key).unwrap();

    unsafe {
        loop {
            eckey = EC_KEY_new_by_curve_name(NID_X9_62_PRIMVE256V1);
            if eckey.is_null() {
                warn!("cannot create EC X9.62 key");
                break;
            }

            ecpt = EC_POINT_hex2point(EC_KEY_get0_group(eckey), native_key.as_ptr(), ptr::null_mut(), ptr::null_mut());
            if ecpt.is_null() {
                warn!("cannot convert raw EC public key to EC point");
                break;
            }

            if EC_KEY_set_public_key(eckey, ecpt) != 1 {
                warn!("cannot set EC public key");
                break;
            }

            peer_key = EVP_PKEY_new();
            if peer_key.is_null() {
                warn!("cannot create EVP pkey");
                break;
            }

            if EVP_PKEY_set1_EC_KEY(peer_key, eckey) != 1 {
                warn!("cannot initialize EVP pkey from EC key");
                EVP_PKEY_free(peer_key);
                peer_key = ptr::null_mut();
                break;
            }

            break;
        }

        if !eckey.is_null() { EC_KEY_free(eckey); }
        if !ecpt.is_null() { EC_POINT_free(ecpt); }
    }

    peer_key
}

/// Generates an OpenSSL representation of the parameters describing
/// an ECDH X9.62 key pair.
fn ecdh_generate_params() -> *mut EvpPkey {
    let param_ctx;
    let mut params = ptr::null_mut();

    unsafe {
        loop {
            param_ctx = EVP_PKEY_CTX_new_id(EVP_PKEY_EC, ptr::null_mut());
            if param_ctx.is_null() {
                warn!("cannot create param context");
                break;
            }

            if EVP_PKEY_paramgen_init(param_ctx) != 1 {
                warn!("cannot init param context");
                break;
            }

            if EVP_PKEY_CTX_ctrl(param_ctx, EVP_PKEY_EC, EVP_PKEY_OP_PARAMGEN, EVP_PKEY_CTRL_EC_PARAMGEN_CURVE_NID, NID_X9_62_PRIMVE256V1, ptr::null_mut()) != 1 {
                warn!("cannot set param context as X9.62 P256V1");
                break;
            }

            if EVP_PKEY_paramgen(param_ctx, &mut params) != 1 || params.is_null() {
                warn!("cannot generate params from context");
                break;
            }

            break;
        }

        if !param_ctx.is_null() { EVP_PKEY_CTX_free(param_ctx); }
    }

    params
}

/// Generates an OpenSSL representation of an ECDH X9.62 key pair.
fn ecdh_generate_key_pair() -> *mut EvpPkey {
    let params;
    let mut key_ctx = ptr::null_mut();
    let mut key = ptr::null_mut();

    unsafe {
        loop {
            params = ecdh_generate_params();
            if params.is_null() {
                break;
            }

            key_ctx = EVP_PKEY_CTX_new(params, ptr::null_mut());
            if key_ctx.is_null() {
                warn!("cannot create key context");
                break;
            }

            if EVP_PKEY_keygen_init(key_ctx) != 1 {
                warn!("cannot init key context");
                break;
            }

            if EVP_PKEY_keygen(key_ctx, &mut key) != 1 || key.is_null() {
                warn!("cannot generate public/private key pair from key context");
                break;
            }

            break;
        }

        if !key_ctx.is_null() { EVP_PKEY_CTX_free(key_ctx); }
        if !params.is_null() { EVP_PKEY_free(params); }
    }

    key
}

/// Derives a shared key from an ECDH X9.62 public/private key pair and
/// another (peer) ECDH X9.62 public key.
fn ecdh_derive_shared_key(local_key: *mut EvpPkey, peer_key: *mut EvpPkey) -> Option<Vec<u8>> {
    let mut status = None;
    let ctx;

    unsafe {
        loop {
            ctx = EVP_PKEY_CTX_new(local_key, ptr::null_mut());
            if ctx.is_null() {
                warn!("cannot create shared key context");
                break;
            }

            if EVP_PKEY_derive_init(ctx) != 1 {
                warn!("cannot init shared context");
                break;
            }

            if EVP_PKEY_derive_set_peer(ctx, peer_key) != 1 {
                warn!("cannot set peer key for shared context");
                break;
            }

            let mut shared_len : libc::size_t = 0;
            if EVP_PKEY_derive(ctx, ptr::null_mut(), &mut shared_len) != 1 {
                warn!("cannot get shared key length from shared context");
                break;
            }

            let shared_key : Vec<u8> = vec![0; shared_len];
            if EVP_PKEY_derive(ctx, shared_key.as_ptr() as *mut libc::c_char, &mut shared_len) != 1 {
                warn!("cannot get shared key from shared context");
                break;
            }

            status = Some(shared_key);
            break;
        }

        if !ctx.is_null() { EVP_PKEY_CTX_free(ctx); }
    }

    status
}

/// Creates a string of hex digits representing an ECDH X9.62 public key
/// given an OpenSSL public/private key pair.
fn ecdh_export_public_key(key: *mut EvpPkey) -> Option<String> {
    let mut status = None;
    let eckey;
    let mut ecpoint = ptr::null_mut();
    let mut pkbuf = ptr::null_mut();

    unsafe {
        loop {
            eckey = EVP_PKEY_get1_EC_KEY(key);
            if eckey.is_null() {
                warn!("cannot get local ec key from local key");
                break;
            }

            ecpoint = EC_KEY_get0_public_key(eckey);
            if ecpoint.is_null() {
                warn!("cannot get public key from local ec key");
                break;
            }

            pkbuf = EC_POINT_point2hex(EC_KEY_get0_group(eckey), ecpoint, EcPointConversion::Uncompressed, ptr::null_mut());
            if pkbuf == ptr::null_mut() {
                warn!("cannot get uncompressed public key from local ec point");
                break;
            }

            status = Some(CStr::from_ptr(pkbuf).to_string_lossy().into_owned());
            break;
        }

        if !pkbuf.is_null() { CRYPTO_free(pkbuf as *mut libc::c_void); }
        if !ecpoint.is_null() { EC_POINT_free(ecpoint); }
        if !eckey.is_null() { EC_KEY_free(eckey); }
    }

    status
}

fn ecdh_derive_keys(raw_peer_key : String) -> Option<EcdhKeyData> {
    let peer_key = ecdh_import_public_key(raw_peer_key);
    let local_key = ecdh_generate_key_pair();
    let shared_key = ecdh_derive_shared_key(local_key, peer_key).unwrap();
    let public_key = ecdh_export_public_key(local_key).unwrap();

    // TODO: if we free both then it crashes, must be some double free happening
    unsafe {
        //if !peer_key.is_null() { EVP_PKEY_free(peer_key); }
        //if !local_key.is_null() { EVP_PKEY_free(local_key); }
    }

    Some(EcdhKeyData {
        public_key: public_key,
        shared_key: shared_key
    })
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
        Some(ekd) => ekd,
        None => {
            warn!("Could not derive keys");
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

