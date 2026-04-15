//! Message parser for converting JSON to typed messages

use crate::errors::{ClaudeError, MessageParseError, Result};
use crate::types::messages::Message;
use serde::Deserialize;

/// Known message types that the SDK can deserialize.
///
/// This list must stay in sync with the variants of `Message`.
/// Test `test_known_message_types_sync` validates this at test time.
pub(crate) const KNOWN_MESSAGE_TYPES: &[&str] = &[
    "assistant",
    "system",
    "result",
    "stream_event",
    "user",
    "control_cancel_request",
    "rate_limit_event",
];

/// Message parser for CLI output
pub struct MessageParser;

impl MessageParser {
    /// Parse a JSON value into a Message, consuming the value.
    ///
    /// Unknown message types return `ClaudeError::UnknownMessageType` instead of
    /// a generic parse error, allowing callers to log and skip them gracefully.
    ///
    /// Missing or non-string `type` fields return `MessageParseError`.
    pub fn parse(data: serde_json::Value) -> Result<Message> {
        // Extract and validate the "type" field
        let type_name = match data.get("type").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => {
                return Err(MessageParseError::new(
                    "Missing or non-string 'type' field",
                    Some(data),
                )
                .into());
            }
        };

        // Check against known types before attempting deserialization
        if !KNOWN_MESSAGE_TYPES.contains(&type_name) {
            return Err(ClaudeError::UnknownMessageType {
                type_name: type_name.to_string(),
                data,
            });
        }

        // Known type — delegate to serde (borrow to preserve data for error reporting)
        Message::deserialize(&data).map_err(|e| {
            MessageParseError::new(format!("Failed to parse message: {}", e), Some(data)).into()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ClaudeError;
    use serde_json::json;

    // T8: Unit tests for MessageParser::parse()

    #[test]
    fn test_parse_missing_type_field() {
        let data = json!({"data": "hello"});
        let result = MessageParser::parse(data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClaudeError::MessageParse(_)));
    }

    #[test]
    fn test_parse_non_string_type_field() {
        let data = json!({"type": 42});
        let result = MessageParser::parse(data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ClaudeError::MessageParse(_)));
    }

    #[test]
    fn test_parse_unknown_type() {
        let data = json!({"type": "some_future_event", "data": {}});
        let result = MessageParser::parse(data);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_unknown_message_type());
        match err {
            ClaudeError::UnknownMessageType { type_name, .. } => {
                assert_eq!(type_name, "some_future_event");
            }
            _ => panic!("Expected UnknownMessageType"),
        }
    }

    // T5: MessageParser returns UnknownMessageType (not generic parse error)
    #[test]
    fn test_unknown_type_returns_unknown_message_type_error() {
        let data = json!({"type": "some_future_event", "data": {}});
        let result = MessageParser::parse(data);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.is_unknown_message_type(),
            "Expected UnknownMessageType, got: {:?}",
            err
        );
        match err {
            ClaudeError::UnknownMessageType { type_name, .. } => {
                assert_eq!(type_name, "some_future_event");
            }
            _ => panic!("Expected UnknownMessageType variant"),
        }
    }

    // T6: Mixed stream with unknown type in the middle completes normally
    #[test]
    fn test_mixed_stream_with_unknown_type() {
        let messages_json = vec![
            json!({
                "type": "system",
                "subtype": "init",
                "session_id": "s1"
            }),
            json!({
                "type": "totally_unknown_future_type",
                "some": "data"
            }),
            json!({
                "type": "result",
                "subtype": "success",
                "duration_ms": 100,
                "duration_api_ms": 80,
                "is_error": false,
                "num_turns": 1,
                "session_id": "s1"
            }),
        ];

        let mut parsed = Vec::new();
        for json in messages_json {
            match MessageParser::parse(json) {
                Ok(msg) => parsed.push(msg),
                Err(e) if e.is_unknown_message_type() => continue,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }

        assert_eq!(parsed.len(), 2);
        assert!(matches!(parsed[0], Message::System(_)));
        assert!(matches!(parsed[1], Message::Result(_)));
    }

    // T7: Known type with bad fields → MessageParseError (not UnknownMessageType)
    #[test]
    fn test_known_type_bad_fields_returns_parse_error() {
        let data = json!({
            "type": "assistant",
            "invalid_field_only": true
        });
        let result = MessageParser::parse(data);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            !err.is_unknown_message_type(),
            "Known type with bad fields should NOT be UnknownMessageType"
        );
        assert!(
            matches!(err, ClaudeError::MessageParse(_)),
            "Expected MessageParse error, got: {:?}",
            err
        );
    }

    // T8b: Verify KNOWN_MESSAGE_TYPES stays in sync with Message enum variants.
    //
    // Constructs a minimal valid instance of each Message variant, serializes it,
    // and checks that the "type" field appears in KNOWN_MESSAGE_TYPES.
    #[test]
    fn test_known_message_types_sync() {
        use crate::types::messages::*;

        // Build one instance of every Message variant
        let variants: Vec<Message> = vec![
            // Assistant
            serde_json::from_value(json!({
                "type": "assistant",
                "message": {
                    "content": [{"type": "text", "text": "hi"}],
                    "model": "test"
                }
            }))
            .unwrap(),
            // System
            serde_json::from_value(json!({
                "type": "system",
                "subtype": "init",
                "session_id": "s"
            }))
            .unwrap(),
            // Result
            serde_json::from_value(json!({
                "type": "result",
                "subtype": "success",
                "duration_ms": 1,
                "duration_api_ms": 1,
                "is_error": false,
                "num_turns": 1,
                "session_id": "s"
            }))
            .unwrap(),
            // StreamEvent
            serde_json::from_value(json!({
                "type": "stream_event",
                "uuid": "u",
                "session_id": "s",
                "event": {}
            }))
            .unwrap(),
            // User
            serde_json::from_value(json!({
                "type": "user"
            }))
            .unwrap(),
            // ControlCancelRequest
            serde_json::from_value(json!({
                "type": "control_cancel_request"
            }))
            .unwrap(),
            // RateLimitEvent
            serde_json::from_value(json!({
                "type": "rate_limit_event",
                "rate_limit_info": {"status": "allowed"}
            }))
            .unwrap(),
        ];

        assert_eq!(
            variants.len(),
            KNOWN_MESSAGE_TYPES.len(),
            "Number of Message variants ({}) must match KNOWN_MESSAGE_TYPES ({})",
            variants.len(),
            KNOWN_MESSAGE_TYPES.len()
        );

        for msg in &variants {
            let serialized = serde_json::to_value(msg).unwrap();
            let type_name = serialized["type"].as_str().unwrap();
            assert!(
                KNOWN_MESSAGE_TYPES.contains(&type_name),
                "Message variant serialized as type '{}' but it's not in KNOWN_MESSAGE_TYPES",
                type_name
            );
        }
    }
}
