use serde::Serialize;

use crate::Result;

pub trait Signable {
    fn signable_bytes(&self) -> Result<Vec<u8>>;
}

pub trait Verifiable {
    fn verify(&self) -> Result<()>;
}

pub trait Addressable {
    fn address(&self) -> &str;
}

pub trait Identifiable {
    fn id(&self) -> &str;
}

pub trait Timestamped {
    fn created_at(&self) -> chrono::DateTime<chrono::Utc>;
}

pub trait Expirable: Timestamped {
    fn expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>>;

    fn is_expired(&self) -> bool {
        self.expires_at()
            .map(|exp| exp < chrono::Utc::now())
            .unwrap_or(false)
    }

    fn ttl(&self) -> Option<chrono::Duration> {
        self.expires_at().map(|exp| exp - chrono::Utc::now())
    }
}

pub trait Persistable: Serialize {
    fn collection() -> &'static str;

    fn key(&self) -> String;

    fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(Into::into)
    }
}

pub trait Mergeable {
    fn merge(&mut self, other: &Self);
}

pub trait Validatable {
    fn validate(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[derive(Debug, Serialize)]
    struct TestRecord {
        id: String,
        value: u64,
        created: chrono::DateTime<chrono::Utc>,
        expires: Option<chrono::DateTime<chrono::Utc>>,
    }

    impl Identifiable for TestRecord {
        fn id(&self) -> &str {
            &self.id
        }
    }

    impl Timestamped for TestRecord {
        fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
            self.created
        }
    }

    impl Expirable for TestRecord {
        fn expires_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
            self.expires
        }
    }

    impl Signable for TestRecord {
        fn signable_bytes(&self) -> Result<Vec<u8>> {
            serde_json::to_vec(self).map_err(Into::into)
        }
    }

    impl Persistable for TestRecord {
        fn collection() -> &'static str {
            "test_records"
        }

        fn key(&self) -> String {
            self.id.clone()
        }
    }

    impl Validatable for TestRecord {
        fn validate(&self) -> Result<()> {
            if self.id.is_empty() {
                return Err(crate::Error::InvalidInput("id cannot be empty".into()));
            }
            Ok(())
        }
    }

    impl Mergeable for TestRecord {
        fn merge(&mut self, other: &Self) {
            if other.value > self.value {
                self.value = other.value;
            }
        }
    }

    #[test]
    fn identifiable_returns_id() {
        let r = TestRecord {
            id: "rec-1".into(),
            value: 42,
            created: Utc::now(),
            expires: None,
        };
        assert_eq!(r.id(), "rec-1");
    }

    #[test]
    fn timestamped_returns_creation_time() {
        let now = Utc::now();
        let r = TestRecord {
            id: "rec-1".into(),
            value: 0,
            created: now,
            expires: None,
        };
        assert_eq!(r.created_at(), now);
    }

    #[test]
    fn expirable_not_expired_when_none() {
        let r = TestRecord {
            id: "rec-1".into(),
            value: 0,
            created: Utc::now(),
            expires: None,
        };
        assert!(!r.is_expired());
        assert!(r.ttl().is_none());
    }

    #[test]
    fn expirable_not_expired_in_future() {
        let r = TestRecord {
            id: "rec-1".into(),
            value: 0,
            created: Utc::now(),
            expires: Some(Utc::now() + Duration::hours(1)),
        };
        assert!(!r.is_expired());
        assert!(r.ttl().unwrap().num_seconds() > 0);
    }

    #[test]
    fn expirable_expired_in_past() {
        let r = TestRecord {
            id: "rec-1".into(),
            value: 0,
            created: Utc::now(),
            expires: Some(Utc::now() - Duration::hours(1)),
        };
        assert!(r.is_expired());
        assert!(r.ttl().unwrap().num_seconds() < 0);
    }

    #[test]
    fn signable_produces_deterministic_bytes() {
        let r = TestRecord {
            id: "rec-1".into(),
            value: 100,
            created: chrono::DateTime::from_timestamp(1000, 0).unwrap(),
            expires: None,
        };
        assert_eq!(r.signable_bytes().unwrap(), r.signable_bytes().unwrap());
    }

    #[test]
    fn persistable_collection_and_key() {
        let r = TestRecord {
            id: "rec-1".into(),
            value: 0,
            created: Utc::now(),
            expires: None,
        };
        assert_eq!(TestRecord::collection(), "test_records");
        assert_eq!(r.key(), "rec-1");
    }

    #[test]
    fn persistable_to_bytes() {
        let r = TestRecord {
            id: "rec-1".into(),
            value: 42,
            created: Utc::now(),
            expires: None,
        };
        let bytes = r.to_bytes().unwrap();
        assert!(!bytes.is_empty());
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["value"], 42);
    }

    #[test]
    fn validatable_accepts_valid() {
        let r = TestRecord {
            id: "rec-1".into(),
            value: 0,
            created: Utc::now(),
            expires: None,
        };
        assert!(r.validate().is_ok());
    }

    #[test]
    fn validatable_rejects_empty_id() {
        let r = TestRecord {
            id: String::new(),
            value: 0,
            created: Utc::now(),
            expires: None,
        };
        assert!(r.validate().is_err());
    }

    #[test]
    fn mergeable_takes_higher_value() {
        let mut a = TestRecord {
            id: "rec-1".into(),
            value: 10,
            created: Utc::now(),
            expires: None,
        };
        let b = TestRecord {
            id: "rec-1".into(),
            value: 20,
            created: Utc::now(),
            expires: None,
        };
        a.merge(&b);
        assert_eq!(a.value, 20);
    }

    #[test]
    fn mergeable_keeps_current_when_higher() {
        let mut a = TestRecord {
            id: "rec-1".into(),
            value: 30,
            created: Utc::now(),
            expires: None,
        };
        let b = TestRecord {
            id: "rec-1".into(),
            value: 10,
            created: Utc::now(),
            expires: None,
        };
        a.merge(&b);
        assert_eq!(a.value, 30);
    }
}
