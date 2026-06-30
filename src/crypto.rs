use anyhow::Result;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::sha2::Sha256;
use rsa::{Oaep, RsaPublicKey};
use serde::Serialize;
use std::collections::HashMap;

/// Wire format for the server's `env_vars` multipart field: a JSON object
/// mapping base64(RSA-OAEP-SHA256(name)) to base64(RSA-OAEP-SHA256(value)).
#[derive(Debug, Serialize)]
pub struct RsaEncryptedEnvVars(pub HashMap<String, String>);

pub fn parse_public_key_pem(pem: &str) -> Result<RsaPublicKey> {
    Ok(RsaPublicKey::from_pkcs1_pem(pem.trim())?)
}

pub fn encrypt_env_vars(
    pub_key: &RsaPublicKey,
    vars: &HashMap<String, String>,
) -> Result<RsaEncryptedEnvVars> {
    use rsa::rand_core::OsRng;
    let mut rng = OsRng;
    let mut out = HashMap::new();
    for (name, value) in vars {
        let enc_name = pub_key.encrypt(&mut rng, Oaep::new::<Sha256>(), name.as_bytes())?;
        let enc_value = pub_key.encrypt(&mut rng, Oaep::new::<Sha256>(), value.as_bytes())?;
        out.insert(
            BASE64_STANDARD.encode(enc_name),
            BASE64_STANDARD.encode(enc_value),
        );
    }
    Ok(RsaEncryptedEnvVars(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::Oaep;
    use rsa::pkcs1::EncodeRsaPublicKey;
    use rsa::rand_core::OsRng;
    use rsa::sha2::Sha256;
    use rsa::{RsaPrivateKey, RsaPublicKey};

    #[test]
    fn parse_rejects_garbage() {
        assert!(parse_public_key_pem("not a pem").is_err());
    }

    #[test]
    fn encrypt_round_trips_through_private_key() {
        // Generate a keypair in-test and serialize the public half to PKCS#1 PEM.
        let mut rng = OsRng;
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pub_key = RsaPublicKey::from(&priv_key);
        let pem = pub_key.to_pkcs1_pem(rsa::pkcs8::LineEnding::LF).unwrap();

        let parsed = parse_public_key_pem(&pem).unwrap();

        let mut vars = HashMap::new();
        vars.insert("DATABASE_URL".to_string(), "postgres://secret".to_string());
        vars.insert("EMPTY".to_string(), "".to_string());

        let encrypted = encrypt_env_vars(&parsed, &vars).unwrap();
        assert_eq!(encrypted.0.len(), 2);

        // Decrypt every name/value pair and rebuild the original map.
        let mut recovered = HashMap::new();
        for (enc_name, enc_value) in &encrypted.0 {
            let name_ct = BASE64_STANDARD.decode(enc_name).unwrap();
            let value_ct = BASE64_STANDARD.decode(enc_value).unwrap();
            let name =
                String::from_utf8(priv_key.decrypt(Oaep::new::<Sha256>(), &name_ct).unwrap())
                    .unwrap();
            let value =
                String::from_utf8(priv_key.decrypt(Oaep::new::<Sha256>(), &value_ct).unwrap())
                    .unwrap();
            recovered.insert(name, value);
        }
        assert_eq!(recovered, vars);
    }

    #[test]
    fn serializes_as_flat_json_object() {
        let mut m = HashMap::new();
        m.insert("k".to_string(), "v".to_string());
        let json = serde_json::to_string(&RsaEncryptedEnvVars(m)).unwrap();
        assert_eq!(json, r#"{"k":"v"}"#);
    }
}
