//! Phase 1 tests: rate_limit_event, PermissionMode, and ResultMessage new fields.
//!
//! Tests T5-T8b (unknown message handling via MessageParser) are unit tests in
//! src/internal/message_parser.rs since MessageParser is internal.

use claude_agent_sdk_rs::types::messages::*;
use serde_json::json;

/// Test helper to load and deserialize a message from filesystem
fn load_fixture(filename: &str) -> Message {
    let path = format!("fixtures/raw_messages/{}", filename);
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Failed to read fixture file: {}", path));
    serde_json::from_str(&json).unwrap_or_else(|_| panic!("Failed to deserialize {}", filename))
}

// ============================================================================
// GROUP 1: RateLimitEvent parsing (T1-T4)
// ============================================================================

/// T1: Full JSON with all fields validated including camelCase renames
#[test]
fn test_rate_limit_event_full_json() {
    let data = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "allowed_warning",
            "resetsAt": 1700000000u64,
            "rateLimitType": "five_hour",
            "utilization": 0.85,
            "isUsingOverage": false,
            "overageStatus": "allowed",
            "overageResetsAt": 1700003600u64,
            "overageDisabledReason": "some_reason"
        },
        "uuid": "550e8400-e29b-41d4-a716-446655440000",
        "session_id": "test-session-id"
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::RateLimitEvent(event) => {
            assert_eq!(
                event.rate_limit_info.status,
                RateLimitStatus::AllowedWarning
            );
            assert_eq!(event.rate_limit_info.resets_at, Some(1700000000));
            assert_eq!(
                event.rate_limit_info.rate_limit_type,
                Some(RateLimitType::FiveHour)
            );
            assert_eq!(event.rate_limit_info.utilization, Some(0.85));
            assert_eq!(event.rate_limit_info.is_using_overage, Some(false));
            assert_eq!(
                event.rate_limit_info.overage_status,
                Some(RateLimitStatus::Allowed)
            );
            assert_eq!(event.rate_limit_info.overage_resets_at, Some(1700003600));
            assert_eq!(
                event.rate_limit_info.overage_disabled_reason,
                Some("some_reason".to_string())
            );
            assert_eq!(
                event.uuid,
                Some("550e8400-e29b-41d4-a716-446655440000".to_string())
            );
            assert_eq!(event.session_id, Some("test-session-id".to_string()));
        }
        _ => panic!("Expected RateLimitEvent"),
    }
}

/// T2: Minimal JSON (only required fields) — optional fields are None
#[test]
fn test_rate_limit_event_minimal_json() {
    let data = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "rejected"
        }
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::RateLimitEvent(event) => {
            assert_eq!(event.rate_limit_info.status, RateLimitStatus::Rejected);
            assert_eq!(event.rate_limit_info.resets_at, None);
            assert_eq!(event.rate_limit_info.rate_limit_type, None);
            assert_eq!(event.rate_limit_info.utilization, None);
            assert_eq!(event.rate_limit_info.is_using_overage, None);
            assert_eq!(event.rate_limit_info.overage_status, None);
            assert_eq!(event.rate_limit_info.overage_resets_at, None);
            assert_eq!(event.rate_limit_info.overage_disabled_reason, None);
            assert_eq!(event.uuid, None);
            assert_eq!(event.session_id, None);
        }
        _ => panic!("Expected RateLimitEvent"),
    }
}

/// T3: Serialize → deserialize roundtrip
#[test]
fn test_rate_limit_event_roundtrip() {
    let data = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "allowed_warning",
            "resetsAt": 1700000000u64,
            "rateLimitType": "seven_day_opus",
            "utilization": 0.92
        },
        "uuid": "roundtrip-uuid",
        "session_id": "roundtrip-session"
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    let serialized = serde_json::to_value(&msg).unwrap();
    let msg2: Message = serde_json::from_value(serialized).unwrap();

    match (msg, msg2) {
        (Message::RateLimitEvent(e1), Message::RateLimitEvent(e2)) => {
            assert_eq!(e1.rate_limit_info.status, e2.rate_limit_info.status);
            assert_eq!(e1.rate_limit_info.resets_at, e2.rate_limit_info.resets_at);
            assert_eq!(
                e1.rate_limit_info.rate_limit_type,
                e2.rate_limit_info.rate_limit_type
            );
            assert_eq!(
                e1.rate_limit_info.utilization,
                e2.rate_limit_info.utilization
            );
            assert_eq!(e1.uuid, e2.uuid);
            assert_eq!(e1.session_id, e2.session_id);
        }
        _ => panic!("Expected RateLimitEvent after roundtrip"),
    }
}

/// T4: Fixture file deserializes correctly
#[test]
fn test_rate_limit_event_fixture() {
    let msg = load_fixture("rate_limit_event_001.json");
    match msg {
        Message::RateLimitEvent(event) => {
            assert_eq!(
                event.rate_limit_info.status,
                RateLimitStatus::AllowedWarning
            );
            assert_eq!(event.rate_limit_info.resets_at, Some(1700000000));
            assert_eq!(
                event.rate_limit_info.rate_limit_type,
                Some(RateLimitType::FiveHour)
            );
            assert_eq!(event.rate_limit_info.utilization, Some(0.85));
            assert_eq!(event.rate_limit_info.is_using_overage, Some(false));
            assert_eq!(event.uuid, Some("abc-123".to_string()));
            assert_eq!(event.session_id, Some("def-456".to_string()));
        }
        _ => panic!("Expected RateLimitEvent from fixture"),
    }
}

// ============================================================================
// Forward compatibility of inner enums
// ============================================================================

#[test]
fn test_rate_limit_status_unknown_value() {
    let data = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "some_future_status"
        }
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::RateLimitEvent(event) => {
            assert_eq!(event.rate_limit_info.status, RateLimitStatus::Unknown);
        }
        _ => panic!("Expected RateLimitEvent"),
    }
}

#[test]
fn test_rate_limit_type_unknown_value() {
    let data = json!({
        "type": "rate_limit_event",
        "rate_limit_info": {
            "status": "allowed",
            "rateLimitType": "some_future_limit_type"
        }
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::RateLimitEvent(event) => {
            assert_eq!(
                event.rate_limit_info.rate_limit_type,
                Some(RateLimitType::Unknown)
            );
        }
        _ => panic!("Expected RateLimitEvent"),
    }
}

// ============================================================================
// GROUP 4: ResultMessage new fields (T12-T16)
// ============================================================================

/// T12: modelUsage (camelCase) maps to model_usage
#[test]
fn test_result_message_model_usage() {
    let data = json!({
        "type": "result",
        "subtype": "success",
        "duration_ms": 100,
        "duration_api_ms": 80,
        "is_error": false,
        "num_turns": 1,
        "session_id": "s1",
        "modelUsage": {
            "claude-sonnet-4-5-20250929": {
                "inputTokens": 100,
                "outputTokens": 50,
                "costUSD": 0.005
            }
        }
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::Result(result) => {
            assert!(result.model_usage.is_some());
            let usage = result.model_usage.unwrap();
            assert_eq!(
                usage["claude-sonnet-4-5-20250929"]["inputTokens"]
                    .as_u64()
                    .unwrap(),
                100
            );
        }
        _ => panic!("Expected Result"),
    }
}

/// T13: permission_denials deserialization
#[test]
fn test_result_message_permission_denials() {
    let data = json!({
        "type": "result",
        "subtype": "success",
        "duration_ms": 100,
        "duration_api_ms": 80,
        "is_error": false,
        "num_turns": 1,
        "session_id": "s1",
        "permission_denials": [
            {"tool": "Write", "reason": "user denied"},
            {"tool": "Bash", "reason": "not allowed"}
        ]
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::Result(result) => {
            assert!(result.permission_denials.is_some());
            let denials = result.permission_denials.unwrap();
            assert_eq!(denials.len(), 2);
            assert_eq!(denials[0]["tool"], "Write");
            assert_eq!(denials[1]["reason"], "not allowed");
        }
        _ => panic!("Expected Result"),
    }
}

/// T14: uuid field capture
#[test]
fn test_result_message_uuid() {
    let data = json!({
        "type": "result",
        "subtype": "success",
        "duration_ms": 100,
        "duration_api_ms": 80,
        "is_error": false,
        "num_turns": 1,
        "session_id": "s1",
        "uuid": "e643c020-beb2-4ef6-ae36-f02b506ea981"
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::Result(result) => {
            assert_eq!(
                result.uuid,
                Some("e643c020-beb2-4ef6-ae36-f02b506ea981".to_string())
            );
        }
        _ => panic!("Expected Result"),
    }
}

/// T15: errors field when present and absent
#[test]
fn test_result_message_errors() {
    // With errors
    let data = json!({
        "type": "result",
        "subtype": "error",
        "duration_ms": 100,
        "duration_api_ms": 80,
        "is_error": true,
        "num_turns": 1,
        "session_id": "s1",
        "errors": ["something went wrong", "another error"]
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::Result(result) => {
            assert!(result.errors.is_some());
            let errors = result.errors.unwrap();
            assert_eq!(errors.len(), 2);
            assert_eq!(errors[0], "something went wrong");
        }
        _ => panic!("Expected Result"),
    }

    // Without errors
    let data = json!({
        "type": "result",
        "subtype": "success",
        "duration_ms": 100,
        "duration_api_ms": 80,
        "is_error": false,
        "num_turns": 1,
        "session_id": "s1"
    });

    let msg: Message = serde_json::from_value(data).unwrap();
    match msg {
        Message::Result(result) => {
            assert!(result.errors.is_none());
        }
        _ => panic!("Expected Result"),
    }
}

/// T16: Re-test result_001-result_006 fixtures — verify new fields captured
#[test]
fn test_result_fixtures_new_fields() {
    for i in 1..=6 {
        let path = format!("result_{:03}.json", i);
        let msg = load_fixture(&path);
        match msg {
            Message::Result(result) => {
                assert!(
                    result.model_usage.is_some(),
                    "result_{:03} should have model_usage",
                    i
                );
                assert!(
                    result.permission_denials.is_some(),
                    "result_{:03} should have permission_denials",
                    i
                );
                assert!(result.uuid.is_some(), "result_{:03} should have uuid", i);
            }
            _ => panic!("Expected Result message in {}", path),
        }
    }
}
