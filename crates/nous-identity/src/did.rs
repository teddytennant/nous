use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use nous_core::Result;
use nous_crypto::keys::{KeyPair, PublicKeyBundle, public_key_to_did};
use nous_crypto::signing::{Signature, Signer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    pub id: String,
    pub r#type: String,
    pub controller: String,
    pub public_key_multibase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    pub verification_method: Vec<VerificationMethod>,
    pub authentication: Vec<String>,
    pub key_agreement: Vec<String>,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
}

impl Document {
    pub fn from_keypair(keypair: &KeyPair) -> Self {
        let did = public_key_to_did(&keypair.verifying_key());
        let now = Utc::now();

        let signing_method = VerificationMethod {
            id: format!("{did}#signing"),
            r#type: "Ed25519VerificationKey2020".to_string(),
            controller: did.clone(),
            public_key_multibase: format!(
                "z{}",
                bs58::encode(keypair.signing_public_bytes()).into_string()
            ),
        };

        let exchange_method = VerificationMethod {
            id: format!("{did}#exchange"),
            r#type: "X25519KeyAgreementKey2020".to_string(),
            controller: did.clone(),
            public_key_multibase: format!(
                "z{}",
                bs58::encode(keypair.exchange_public_bytes()).into_string()
            ),
        };

        Self {
            context: vec![
                "https://www.w3.org/ns/did/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
                "https://w3id.org/security/suites/x25519-2020/v1".to_string(),
            ],
            id: did.clone(),
            authentication: vec![format!("{did}#signing")],
            key_agreement: vec![format!("{did}#exchange")],
            verification_method: vec![signing_method, exchange_method],
            created: now,
            updated: now,
        }
    }

    pub fn did(&self) -> &str {
        &self.id
    }
}

#[derive(Serialize, Deserialize)]
pub struct Identity {
    keypair: KeyPair,
    document: Document,
    display_name: Option<String>,
}

impl Identity {
    pub fn generate() -> Self {
        let keypair = KeyPair::generate();
        let document = Document::from_keypair(&keypair);
        Self {
            keypair,
            document,
            display_name: None,
        }
    }

    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    pub fn did(&self) -> &str {
        self.document.did()
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn keypair(&self) -> &KeyPair {
        &self.keypair
    }

    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    pub fn public_bundle(&self) -> PublicKeyBundle {
        self.keypair.public_bundle()
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        let signer = Signer::new(&self.keypair);
        signer.sign(message)
    }

    pub fn sign_json<T: Serialize>(&self, value: &T) -> Result<Signature> {
        let signer = Signer::new(&self.keypair);
        signer.sign_json(value)
    }

    pub fn export_signing_key(&self) -> [u8; 32] {
        self.keypair.signing_secret_bytes()
    }

    pub fn restore(signing_bytes: &[u8]) -> Result<Self> {
        let keypair = KeyPair::from_signing_bytes(signing_bytes)?;
        let document = Document::from_keypair(&keypair);
        Ok(Self {
            keypair,
            document,
            display_name: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nous_crypto::signing::Verifier;

    #[test]
    fn identity_generate_produces_valid_did() {
        let id = Identity::generate();
        assert!(id.did().starts_with("did:key:z"));
    }

    #[test]
    fn identity_display_name() {
        let id = Identity::generate().with_display_name("Zarathustra");
        assert_eq!(id.display_name(), Some("Zarathustra"));
    }

    #[test]
    fn identity_sign_and_verify() {
        let id = Identity::generate();
        let message = b"what does not kill me makes me stronger";
        let sig = id.sign(message);

        assert!(Verifier::verify(&id.keypair().verifying_key(), message, &sig).is_ok());
    }

    #[test]
    fn identity_export_restore_roundtrip() {
        let id = Identity::generate();
        let did = id.did().to_string();
        let key_bytes = id.export_signing_key();

        let restored = Identity::restore(&key_bytes).unwrap();
        assert_eq!(restored.did(), did);
    }

    #[test]
    fn identity_restored_can_sign() {
        let original = Identity::generate();
        let key_bytes = original.export_signing_key();
        let restored = Identity::restore(&key_bytes).unwrap();

        let message = b"persistence test";
        let sig = restored.sign(message);

        assert!(Verifier::verify(&original.keypair().verifying_key(), message, &sig).is_ok());
    }

    #[test]
    fn document_has_correct_structure() {
        let id = Identity::generate();
        let doc = id.document();

        assert_eq!(doc.context.len(), 3);
        assert_eq!(doc.verification_method.len(), 2);
        assert_eq!(doc.authentication.len(), 1);
        assert_eq!(doc.key_agreement.len(), 1);
        assert!(doc.authentication[0].ends_with("#signing"));
        assert!(doc.key_agreement[0].ends_with("#exchange"));
    }

    #[test]
    fn document_verification_methods_have_correct_types() {
        let id = Identity::generate();
        let doc = id.document();

        assert_eq!(
            doc.verification_method[0].r#type,
            "Ed25519VerificationKey2020"
        );
        assert_eq!(
            doc.verification_method[1].r#type,
            "X25519KeyAgreementKey2020"
        );
    }

    #[test]
    fn document_serializes_to_valid_json() {
        let id = Identity::generate();
        let json = serde_json::to_string_pretty(id.document()).unwrap();

        assert!(json.contains("\"@context\""));
        assert!(json.contains("did:key:z"));
        assert!(json.contains("Ed25519VerificationKey2020"));

        // deserializes back
        let _: Document = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn two_identities_have_different_dids() {
        let a = Identity::generate();
        let b = Identity::generate();
        assert_ne!(a.did(), b.did());
    }

    #[test]
    fn identity_serde_roundtrip() {
        let id = Identity::generate().with_display_name("Zarathustra");
        let did = id.did().to_string();
        let signing_pub = id.keypair().signing_public_bytes();

        let json = serde_json::to_string(&id).unwrap();
        let restored: Identity = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.did(), did);
        assert_eq!(restored.display_name(), Some("Zarathustra"));
        assert_eq!(restored.keypair().signing_public_bytes(), signing_pub);
    }

    #[test]
    fn identity_serde_preserves_signing() {
        let id = Identity::generate();
        let json = serde_json::to_string(&id).unwrap();
        let restored: Identity = serde_json::from_str(&json).unwrap();

        let message = b"persistence signing test";
        let sig = restored.sign(message);
        assert!(Verifier::verify(&id.keypair().verifying_key(), message, &sig).is_ok());
    }
}
