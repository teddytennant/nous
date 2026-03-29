use crate::event::Event;
use crate::filter::Filter;
use thiserror::Error;

/// Errors during message parsing.
#[derive(Debug, Error)]
pub enum MessageError {
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("unknown message type: {0}")]
    UnknownType(String),

    #[error("malformed message: {0}")]
    Malformed(String),
}

/// Client-to-relay messages per NIP-01.
#[derive(Debug, Clone, PartialEq)]
pub enum ClientMessage {
    /// `["EVENT", <event>]`
    Event(Event),
    /// `["REQ", <subscription_id>, <filter>, ...]`
    Req {
        subscription_id: String,
        filters: Vec<Filter>,
    },
    /// `["CLOSE", <subscription_id>]`
    Close(String),
}

/// Relay-to-client messages per NIP-01.
#[derive(Debug, Clone, PartialEq)]
pub enum RelayMessage {
    /// `["EVENT", <subscription_id>, <event>]`
    Event {
        subscription_id: String,
        event: Event,
    },
    /// `["EOSE", <subscription_id>]`
    EndOfStoredEvents(String),
    /// `["NOTICE", <message>]`
    Notice(String),
    /// `["OK", <event_id>, <accepted>, <message>]`
    Ok {
        event_id: String,
        accepted: bool,
        message: String,
    },
}

impl ClientMessage {
    /// Parse a client message from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, MessageError> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let arr = value
            .as_array()
            .ok_or_else(|| MessageError::Malformed("expected JSON array".into()))?;

        let msg_type = arr
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| MessageError::Malformed("missing message type".into()))?;

        match msg_type {
            "EVENT" => {
                let event: Event = serde_json::from_value(
                    arr.get(1)
                        .cloned()
                        .ok_or_else(|| MessageError::Malformed("missing event".into()))?,
                )?;
                Ok(ClientMessage::Event(event))
            }
            "REQ" => {
                let sub_id = arr
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MessageError::Malformed("missing subscription id".into()))?
                    .to_string();

                let filters: Vec<Filter> = arr[2..]
                    .iter()
                    .map(|v| serde_json::from_value(v.clone()))
                    .collect::<Result<_, _>>()?;

                if filters.is_empty() {
                    return Err(MessageError::Malformed(
                        "REQ must have at least one filter".into(),
                    ));
                }

                Ok(ClientMessage::Req {
                    subscription_id: sub_id,
                    filters,
                })
            }
            "CLOSE" => {
                let sub_id = arr
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MessageError::Malformed("missing subscription id".into()))?
                    .to_string();
                Ok(ClientMessage::Close(sub_id))
            }
            other => Err(MessageError::UnknownType(other.to_string())),
        }
    }

    /// Serialize to a JSON string.
    pub fn to_json(&self) -> String {
        match self {
            ClientMessage::Event(event) => {
                let event_json = serde_json::to_value(event).unwrap();
                serde_json::to_string(&serde_json::json!(["EVENT", event_json])).unwrap()
            }
            ClientMessage::Req {
                subscription_id,
                filters,
            } => {
                let mut arr: Vec<serde_json::Value> = vec![
                    serde_json::json!("REQ"),
                    serde_json::json!(subscription_id),
                ];
                for f in filters {
                    arr.push(serde_json::to_value(f).unwrap());
                }
                serde_json::to_string(&arr).unwrap()
            }
            ClientMessage::Close(sub_id) => {
                serde_json::to_string(&serde_json::json!(["CLOSE", sub_id])).unwrap()
            }
        }
    }
}

impl RelayMessage {
    /// Parse a relay message from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, MessageError> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let arr = value
            .as_array()
            .ok_or_else(|| MessageError::Malformed("expected JSON array".into()))?;

        let msg_type = arr
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| MessageError::Malformed("missing message type".into()))?;

        match msg_type {
            "EVENT" => {
                let sub_id = arr
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MessageError::Malformed("missing subscription id".into()))?
                    .to_string();
                let event: Event = serde_json::from_value(
                    arr.get(2)
                        .cloned()
                        .ok_or_else(|| MessageError::Malformed("missing event".into()))?,
                )?;
                Ok(RelayMessage::Event {
                    subscription_id: sub_id,
                    event,
                })
            }
            "EOSE" => {
                let sub_id = arr
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MessageError::Malformed("missing subscription id".into()))?
                    .to_string();
                Ok(RelayMessage::EndOfStoredEvents(sub_id))
            }
            "NOTICE" => {
                let msg = arr
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MessageError::Malformed("missing notice message".into()))?
                    .to_string();
                Ok(RelayMessage::Notice(msg))
            }
            "OK" => {
                let event_id = arr
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| MessageError::Malformed("missing event id".into()))?
                    .to_string();
                let accepted = arr
                    .get(2)
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| MessageError::Malformed("missing accepted flag".into()))?;
                let message = arr
                    .get(3)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(RelayMessage::Ok {
                    event_id,
                    accepted,
                    message,
                })
            }
            other => Err(MessageError::UnknownType(other.to_string())),
        }
    }

    /// Serialize to a JSON string.
    pub fn to_json(&self) -> String {
        match self {
            RelayMessage::Event {
                subscription_id,
                event,
            } => {
                let event_json = serde_json::to_value(event).unwrap();
                serde_json::to_string(
                    &serde_json::json!(["EVENT", subscription_id, event_json]),
                )
                .unwrap()
            }
            RelayMessage::EndOfStoredEvents(sub_id) => {
                serde_json::to_string(&serde_json::json!(["EOSE", sub_id])).unwrap()
            }
            RelayMessage::Notice(msg) => {
                serde_json::to_string(&serde_json::json!(["NOTICE", msg])).unwrap()
            }
            RelayMessage::Ok {
                event_id,
                accepted,
                message,
            } => serde_json::to_string(
                &serde_json::json!(["OK", event_id, accepted, message]),
            )
            .unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventBuilder, Kind, Tag};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn test_event() -> Event {
        let key = SigningKey::generate(&mut OsRng);
        EventBuilder::text_note("hello")
            .created_at(1000)
            .sign(&key)
    }

    #[test]
    fn client_event_roundtrip() {
        let event = test_event();
        let msg = ClientMessage::Event(event.clone());
        let json = msg.to_json();
        let parsed = ClientMessage::from_json(&json).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn client_req_roundtrip() {
        let msg = ClientMessage::Req {
            subscription_id: "sub1".into(),
            filters: vec![Filter::new().kinds(vec![Kind::TEXT_NOTE]).since(1000)],
        };
        let json = msg.to_json();
        let parsed = ClientMessage::from_json(&json).unwrap();
        if let ClientMessage::Req {
            subscription_id,
            filters,
        } = parsed
        {
            assert_eq!(subscription_id, "sub1");
            assert_eq!(filters.len(), 1);
            assert_eq!(filters[0].kinds.as_ref().unwrap(), &[Kind::TEXT_NOTE]);
        } else {
            panic!("expected Req");
        }
    }

    #[test]
    fn client_close_roundtrip() {
        let msg = ClientMessage::Close("sub1".into());
        let json = msg.to_json();
        let parsed = ClientMessage::from_json(&json).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn relay_event_roundtrip() {
        let event = test_event();
        let msg = RelayMessage::Event {
            subscription_id: "sub1".into(),
            event: event.clone(),
        };
        let json = msg.to_json();
        let parsed = RelayMessage::from_json(&json).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn relay_eose_roundtrip() {
        let msg = RelayMessage::EndOfStoredEvents("sub1".into());
        let json = msg.to_json();
        let parsed = RelayMessage::from_json(&json).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn relay_notice_roundtrip() {
        let msg = RelayMessage::Notice("rate limited".into());
        let json = msg.to_json();
        let parsed = RelayMessage::from_json(&json).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn relay_ok_roundtrip() {
        let msg = RelayMessage::Ok {
            event_id: "abc123".into(),
            accepted: true,
            message: "".into(),
        };
        let json = msg.to_json();
        let parsed = RelayMessage::from_json(&json).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn relay_ok_rejected() {
        let msg = RelayMessage::Ok {
            event_id: "abc123".into(),
            accepted: false,
            message: "duplicate: already have this event".into(),
        };
        let json = msg.to_json();
        let parsed = RelayMessage::from_json(&json).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn parse_invalid_json() {
        assert!(ClientMessage::from_json("not json").is_err());
    }

    #[test]
    fn parse_non_array() {
        assert!(ClientMessage::from_json(r#"{"type":"EVENT"}"#).is_err());
    }

    #[test]
    fn parse_unknown_type() {
        let err = ClientMessage::from_json(r#"["UNKNOWN", "data"]"#).unwrap_err();
        assert!(matches!(err, MessageError::UnknownType(_)));
    }

    #[test]
    fn parse_req_without_filters() {
        let err = ClientMessage::from_json(r#"["REQ", "sub1"]"#).unwrap_err();
        assert!(matches!(err, MessageError::Malformed(_)));
    }

    #[test]
    fn client_req_multiple_filters() {
        let msg = ClientMessage::Req {
            subscription_id: "multi".into(),
            filters: vec![
                Filter::new().kinds(vec![Kind::TEXT_NOTE]),
                Filter::new().kinds(vec![Kind::METADATA]),
            ],
        };
        let json = msg.to_json();
        let parsed = ClientMessage::from_json(&json).unwrap();
        if let ClientMessage::Req { filters, .. } = parsed {
            assert_eq!(filters.len(), 2);
        } else {
            panic!("expected Req");
        }
    }
}
