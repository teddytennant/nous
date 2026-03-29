use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use nous_core::{Error, Result};
use nous_crypto::signing::{Signature, Signer, Verifier};

use crate::credential::Credential;
use crate::did::Identity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisclosureRequest {
    pub id: String,
    pub verifier_did: String,
    pub required_types: Vec<String>,
    pub required_fields: Vec<String>,
    pub purpose: String,
    pub created_at: DateTime<Utc>,
    pub nonce: String,
}

impl DisclosureRequest {
    pub fn new(verifier_did: &str, purpose: &str) -> Self {
        Self {
            id: format!("dreq:{}", Uuid::new_v4()),
            verifier_did: verifier_did.into(),
            required_types: Vec::new(),
            required_fields: Vec::new(),
            purpose: purpose.into(),
            created_at: Utc::now(),
            nonce: Uuid::new_v4().to_string(),
        }
    }

    pub fn require_type(mut self, credential_type: impl Into<String>) -> Self {
        self.required_types.push(credential_type.into());
        self
    }

    pub fn require_field(mut self, field: impl Into<String>) -> Self {
        self.required_fields.push(field.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectiveDisclosure {
    pub id: String,
    pub request_id: String,
    pub holder_did: String,
    pub disclosed_claims: serde_json::Value,
    pub credential_type: Vec<String>,
    pub issuer_did: String,
    pub issuance_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub nonce: String,
    pub proof: Option<Signature>,
}

impl SelectiveDisclosure {
    pub fn from_credential(
        credential: &Credential,
        request: &DisclosureRequest,
        fields: &HashSet<String>,
        holder: &Identity,
    ) -> Result<Self> {
        // Verify the credential is valid
        credential.verify()?;

        // Check required types
        for req_type in &request.required_types {
            if !credential.r#type.contains(req_type) {
                return Err(Error::InvalidInput(format!(
                    "credential missing required type: {req_type}"
                )));
            }
        }

        // Extract only the requested fields
        let claims = &credential.credential_subject.claims;
        let disclosed = Self::filter_claims(claims, fields)?;

        // Verify all required fields are present
        for field in &request.required_fields {
            if disclosed.get(field).is_none() {
                return Err(Error::InvalidInput(format!(
                    "credential missing required field: {field}"
                )));
            }
        }

        let mut disclosure = Self {
            id: format!("disc:{}", Uuid::new_v4()),
            request_id: request.id.clone(),
            holder_did: holder.did().to_string(),
            disclosed_claims: disclosed,
            credential_type: credential.r#type.clone(),
            issuer_did: credential.issuer.clone(),
            issuance_date: credential.issuance_date,
            created_at: Utc::now(),
            nonce: request.nonce.clone(),
            proof: None,
        };

        // Sign the disclosure
        let payload = disclosure.signable_bytes()?;
        let signer = Signer::new(holder.keypair());
        disclosure.proof = Some(signer.sign(&payload));

        Ok(disclosure)
    }

    fn filter_claims(
        claims: &serde_json::Value,
        fields: &HashSet<String>,
    ) -> Result<serde_json::Value> {
        match claims {
            serde_json::Value::Object(map) => {
                let filtered: serde_json::Map<String, serde_json::Value> = map
                    .iter()
                    .filter(|(key, _)| fields.contains(key.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Ok(serde_json::Value::Object(filtered))
            }
            _ => Err(Error::InvalidInput("claims must be an object".into())),
        }
    }

    fn signable_bytes(&self) -> Result<Vec<u8>> {
        let signable = serde_json::json!({
            "id": self.id,
            "request_id": self.request_id,
            "holder_did": self.holder_did,
            "disclosed_claims": self.disclosed_claims,
            "issuer_did": self.issuer_did,
            "nonce": self.nonce,
        });
        serde_json::to_vec(&signable).map_err(Into::into)
    }

    pub fn verify(&self) -> Result<()> {
        let proof = self
            .proof
            .as_ref()
            .ok_or_else(|| Error::Crypto("disclosure has no proof".into()))?;

        let holder_key = nous_crypto::keys::did_to_public_key(&self.holder_did)?;
        let payload = self.signable_bytes()?;
        Verifier::verify(&holder_key, &payload, proof)
    }

    pub fn disclosed_field(&self, field: &str) -> Option<&serde_json::Value> {
        self.disclosed_claims.get(field)
    }

    pub fn field_count(&self) -> usize {
        match &self.disclosed_claims {
            serde_json::Value::Object(map) => map.len(),
            _ => 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct PresentationBuilder {
    disclosures: Vec<SelectiveDisclosure>,
}

impl PresentationBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, disclosure: SelectiveDisclosure) -> Self {
        self.disclosures.push(disclosure);
        self
    }

    pub fn build(self) -> Presentation {
        Presentation {
            id: format!("pres:{}", Uuid::new_v4()),
            disclosures: self.disclosures,
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presentation {
    pub id: String,
    pub disclosures: Vec<SelectiveDisclosure>,
    pub created_at: DateTime<Utc>,
}

impl Presentation {
    pub fn verify_all(&self) -> Result<()> {
        for disclosure in &self.disclosures {
            disclosure.verify()?;
        }
        Ok(())
    }

    pub fn disclosure_count(&self) -> usize {
        self.disclosures.len()
    }

    pub fn holder_dids(&self) -> Vec<&str> {
        self.disclosures
            .iter()
            .map(|d| d.holder_did.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credential::CredentialBuilder;

    fn test_credential(issuer: &Identity, subject: &Identity) -> Credential {
        CredentialBuilder::new(subject.did())
            .add_type("MembershipCredential")
            .claims(serde_json::json!({
                "role": "validator",
                "level": 3,
                "region": "EU",
                "joined": "2024-01-15"
            }))
            .issue(issuer)
            .unwrap()
    }

    #[test]
    fn disclosure_request_creation() {
        let req = DisclosureRequest::new("did:key:zverifier", "age verification")
            .require_type("MembershipCredential")
            .require_field("role");

        assert!(req.id.starts_with("dreq:"));
        assert_eq!(req.required_types.len(), 1);
        assert_eq!(req.required_fields.len(), 1);
    }

    #[test]
    fn selective_disclosure_reveals_only_requested_fields() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let request = DisclosureRequest::new("did:key:zverifier", "role check")
            .require_type("MembershipCredential")
            .require_field("role");

        let fields: HashSet<String> = ["role", "level"].iter().map(|s| s.to_string()).collect();

        let disclosure =
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder)
                .unwrap();

        assert_eq!(disclosure.field_count(), 2);
        assert_eq!(
            disclosure.disclosed_field("role"),
            Some(&serde_json::json!("validator"))
        );
        assert_eq!(
            disclosure.disclosed_field("level"),
            Some(&serde_json::json!(3))
        );
        assert!(disclosure.disclosed_field("region").is_none());
        assert!(disclosure.disclosed_field("joined").is_none());
    }

    #[test]
    fn disclosure_is_verifiable() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let request = DisclosureRequest::new("did:key:zverifier", "check");
        let fields: HashSet<String> = ["role"].iter().map(|s| s.to_string()).collect();

        let disclosure =
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder)
                .unwrap();

        assert!(disclosure.verify().is_ok());
    }

    #[test]
    fn tampered_disclosure_fails_verification() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let request = DisclosureRequest::new("did:key:zverifier", "check");
        let fields: HashSet<String> = ["role"].iter().map(|s| s.to_string()).collect();

        let mut disclosure =
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder)
                .unwrap();

        // Tamper with disclosed claims
        disclosure.disclosed_claims = serde_json::json!({"role": "admin"});

        assert!(disclosure.verify().is_err());
    }

    #[test]
    fn missing_required_type_fails() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let request = DisclosureRequest::new("did:key:zverifier", "check")
            .require_type("AgeCredential");

        let fields: HashSet<String> = ["role"].iter().map(|s| s.to_string()).collect();

        assert!(
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder).is_err()
        );
    }

    #[test]
    fn missing_required_field_fails() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let request = DisclosureRequest::new("did:key:zverifier", "check")
            .require_type("MembershipCredential")
            .require_field("nonexistent_field");

        let fields: HashSet<String> = ["role"].iter().map(|s| s.to_string()).collect();

        assert!(
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder).is_err()
        );
    }

    #[test]
    fn expired_credential_fails_disclosure() {
        let issuer = Identity::generate();
        let holder = Identity::generate();

        let credential = CredentialBuilder::new(holder.did())
            .add_type("TestCredential")
            .claims(serde_json::json!({"key": "value"}))
            .expires_at(Utc::now() - chrono::Duration::hours(1))
            .issue(&issuer)
            .unwrap();

        let request = DisclosureRequest::new("did:key:zverifier", "check");
        let fields: HashSet<String> = ["key"].iter().map(|s| s.to_string()).collect();

        assert!(
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder).is_err()
        );
    }

    #[test]
    fn presentation_with_multiple_disclosures() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let req1 = DisclosureRequest::new("did:key:zverifier", "role check");
        let req2 = DisclosureRequest::new("did:key:zverifier", "region check");

        let fields1: HashSet<String> = ["role"].iter().map(|s| s.to_string()).collect();
        let fields2: HashSet<String> = ["region"].iter().map(|s| s.to_string()).collect();

        let d1 =
            SelectiveDisclosure::from_credential(&credential, &req1, &fields1, &holder).unwrap();
        let d2 =
            SelectiveDisclosure::from_credential(&credential, &req2, &fields2, &holder).unwrap();

        let presentation = PresentationBuilder::new().with(d1).with(d2).build();

        assert_eq!(presentation.disclosure_count(), 2);
        assert!(presentation.verify_all().is_ok());
    }

    #[test]
    fn presentation_holder_dids() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let request = DisclosureRequest::new("did:key:zverifier", "check");
        let fields: HashSet<String> = ["role"].iter().map(|s| s.to_string()).collect();
        let d =
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder).unwrap();

        let presentation = PresentationBuilder::new().with(d).build();
        let dids = presentation.holder_dids();
        assert_eq!(dids.len(), 1);
        assert_eq!(dids[0], holder.did());
    }

    #[test]
    fn disclosure_serializes() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let request = DisclosureRequest::new("did:key:zverifier", "check");
        let fields: HashSet<String> = ["role", "level"].iter().map(|s| s.to_string()).collect();

        let disclosure =
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder)
                .unwrap();

        let json = serde_json::to_string(&disclosure).unwrap();
        let restored: SelectiveDisclosure = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.field_count(), 2);
        assert!(restored.verify().is_ok());
    }

    #[test]
    fn disclosure_request_serializes() {
        let req = DisclosureRequest::new("did:key:zverifier", "test")
            .require_type("TestType")
            .require_field("name");

        let json = serde_json::to_string(&req).unwrap();
        let restored: DisclosureRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.purpose, "test");
    }

    #[test]
    fn presentation_serializes() {
        let issuer = Identity::generate();
        let holder = Identity::generate();
        let credential = test_credential(&issuer, &holder);

        let request = DisclosureRequest::new("did:key:zverifier", "check");
        let fields: HashSet<String> = ["role"].iter().map(|s| s.to_string()).collect();
        let d =
            SelectiveDisclosure::from_credential(&credential, &request, &fields, &holder).unwrap();

        let presentation = PresentationBuilder::new().with(d).build();
        let json = serde_json::to_string(&presentation).unwrap();
        let restored: Presentation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.disclosure_count(), 1);
        assert!(restored.verify_all().is_ok());
    }
}
