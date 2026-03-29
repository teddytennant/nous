use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};
use nous_crypto::keys::did_to_public_key;
use nous_crypto::signing::{Signature, Verifier};

use crate::did::Identity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSubject {
    pub id: String,
    #[serde(flatten)]
    pub claims: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialProof {
    pub r#type: String,
    pub created: DateTime<Utc>,
    pub verification_method: String,
    pub proof_purpose: String,
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    pub id: String,
    pub r#type: Vec<String>,
    pub issuer: String,
    pub issuance_date: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<DateTime<Utc>>,
    pub credential_subject: CredentialSubject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<CredentialProof>,
}

impl Credential {
    pub fn is_expired(&self) -> bool {
        self.expiration_date
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }

    pub fn verify(&self) -> Result<()> {
        if self.is_expired() {
            return Err(Error::Expired("credential has expired".into()));
        }

        let proof = self
            .proof
            .as_ref()
            .ok_or_else(|| Error::Crypto("credential has no proof".into()))?;

        let issuer_key = did_to_public_key(&self.issuer)?;

        let mut unsigned = self.clone();
        unsigned.proof = None;
        let payload = serde_json::to_vec(&unsigned)?;

        Verifier::verify(&issuer_key, &payload, &proof.signature)
    }

    pub fn subject_did(&self) -> &str {
        &self.credential_subject.id
    }

    pub fn issuer_did(&self) -> &str {
        &self.issuer
    }
}

pub struct CredentialBuilder {
    types: Vec<String>,
    subject_did: String,
    claims: serde_json::Value,
    expiration: Option<DateTime<Utc>>,
}

impl CredentialBuilder {
    pub fn new(subject_did: impl Into<String>) -> Self {
        Self {
            types: vec!["VerifiableCredential".to_string()],
            subject_did: subject_did.into(),
            claims: serde_json::json!({}),
            expiration: None,
        }
    }

    pub fn add_type(mut self, credential_type: impl Into<String>) -> Self {
        self.types.push(credential_type.into());
        self
    }

    pub fn claims(mut self, claims: serde_json::Value) -> Self {
        self.claims = claims;
        self
    }

    pub fn expires_at(mut self, expiration: DateTime<Utc>) -> Self {
        self.expiration = Some(expiration);
        self
    }

    pub fn issue(self, issuer: &Identity) -> Result<Credential> {
        let mut credential = Credential {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://www.w3.org/2018/credentials/examples/v1".to_string(),
            ],
            id: format!("urn:uuid:{}", Uuid::new_v4()),
            r#type: self.types,
            issuer: issuer.did().to_string(),
            issuance_date: Utc::now(),
            expiration_date: self.expiration,
            credential_subject: CredentialSubject {
                id: self.subject_did,
                claims: self.claims,
            },
            proof: None,
        };

        let payload = serde_json::to_vec(&credential)?;
        let signature = issuer.sign(&payload);

        credential.proof = Some(CredentialProof {
            r#type: "Ed25519Signature2020".to_string(),
            created: Utc::now(),
            verification_method: format!("{}#signing", issuer.did()),
            proof_purpose: "assertionMethod".to_string(),
            signature,
        });

        Ok(credential)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn issue_and_verify_credential() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let credential = CredentialBuilder::new(subject.did())
            .add_type("MembershipCredential")
            .claims(serde_json::json!({"role": "validator", "level": 3}))
            .issue(&issuer)
            .unwrap();

        assert!(credential.verify().is_ok());
    }

    #[test]
    fn credential_has_correct_metadata() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let credential = CredentialBuilder::new(subject.did())
            .add_type("ReputationCredential")
            .claims(serde_json::json!({"score": 95}))
            .issue(&issuer)
            .unwrap();

        assert_eq!(credential.issuer_did(), issuer.did());
        assert_eq!(credential.subject_did(), subject.did());
        assert!(
            credential
                .r#type
                .contains(&"VerifiableCredential".to_string())
        );
        assert!(
            credential
                .r#type
                .contains(&"ReputationCredential".to_string())
        );
        assert!(credential.id.starts_with("urn:uuid:"));
    }

    #[test]
    fn expired_credential_fails_verification() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let credential = CredentialBuilder::new(subject.did())
            .expires_at(Utc::now() - Duration::hours(1))
            .issue(&issuer)
            .unwrap();

        assert!(credential.is_expired());
        let err = credential.verify().unwrap_err();
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn non_expired_credential_passes() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let credential = CredentialBuilder::new(subject.did())
            .expires_at(Utc::now() + Duration::days(365))
            .issue(&issuer)
            .unwrap();

        assert!(!credential.is_expired());
        assert!(credential.verify().is_ok());
    }

    #[test]
    fn tampered_credential_fails_verification() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let mut credential = CredentialBuilder::new(subject.did())
            .claims(serde_json::json!({"access": "read"}))
            .issue(&issuer)
            .unwrap();

        // tamper with claims
        credential.credential_subject.claims = serde_json::json!({"access": "admin"});

        assert!(credential.verify().is_err());
    }

    #[test]
    fn credential_serializes_to_valid_json() {
        let issuer = Identity::generate();
        let subject = Identity::generate();

        let credential = CredentialBuilder::new(subject.did())
            .add_type("TestCredential")
            .claims(serde_json::json!({"test": true}))
            .issue(&issuer)
            .unwrap();

        let json = serde_json::to_string_pretty(&credential).unwrap();
        assert!(json.contains("@context"));
        assert!(json.contains("VerifiableCredential"));
        assert!(json.contains("Ed25519Signature2020"));

        let deserialized: Credential = serde_json::from_str(&json).unwrap();
        assert!(deserialized.verify().is_ok());
    }

    #[test]
    fn self_issued_credential() {
        let identity = Identity::generate();

        let credential = CredentialBuilder::new(identity.did())
            .add_type("SelfDeclaration")
            .claims(serde_json::json!({"name": "Zarathustra"}))
            .issue(&identity)
            .unwrap();

        assert_eq!(credential.issuer_did(), credential.subject_did());
        assert!(credential.verify().is_ok());
    }
}
