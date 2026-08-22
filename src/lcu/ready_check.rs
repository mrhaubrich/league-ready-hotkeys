use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use super::READY_CHECK;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadyCheckResponse {
    None,
    Accepted,
    Declined,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyCheck {
    pub active: bool,
    pub response: ReadyCheckResponse,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReadyCheckError {
    #[error("ready-check payload is not an object")]
    NotObject,
    #[error("ready-check state is missing or invalid")]
    InvalidState,
    #[error("ready-check player response is invalid")]
    InvalidResponse,
    #[error("event payload is not a ready-check update")]
    NotReadyCheckEvent,
}

#[derive(Deserialize)]
struct ReadyCheckPayload {
    state: String,
    #[serde(rename = "playerResponse")]
    player_response: String,
}

pub fn parse_ready_check(value: &Value) -> Result<ReadyCheck, ReadyCheckError> {
    let payload: ReadyCheckPayload = serde_json::from_value(value.clone()).map_err(|error| {
        if value.is_object() && error.to_string().contains("playerResponse") {
            ReadyCheckError::InvalidResponse
        } else if value.is_object() {
            ReadyCheckError::InvalidState
        } else {
            ReadyCheckError::NotObject
        }
    })?;
    let response = match payload.player_response.as_str() {
        "None" => ReadyCheckResponse::None,
        "Accepted" => ReadyCheckResponse::Accepted,
        "Declined" => ReadyCheckResponse::Declined,
        _ => return Err(ReadyCheckError::InvalidResponse),
    };
    Ok(ReadyCheck {
        active: payload.state == "InProgress" && response == ReadyCheckResponse::None,
        response,
    })
}

pub fn parse_ready_check_event(event: &Value) -> Result<ReadyCheck, ReadyCheckError> {
    let object = event
        .as_object()
        .ok_or(ReadyCheckError::NotReadyCheckEvent)?;
    if object.get("uri").and_then(Value::as_str) != Some(READY_CHECK) {
        return Err(ReadyCheckError::NotReadyCheckEvent);
    }
    parse_ready_check(
        object
            .get("data")
            .ok_or(ReadyCheckError::NotReadyCheckEvent)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_unanswered_in_progress_is_active() {
        let value = json!({"state":"InProgress","playerResponse":"None"});
        assert_eq!(
            parse_ready_check(&value).unwrap(),
            ReadyCheck {
                active: true,
                response: ReadyCheckResponse::None
            }
        );
        let accepted = json!({"state":"InProgress","playerResponse":"Accepted"});
        assert!(!parse_ready_check(&accepted).unwrap().active);
    }

    #[test]
    fn parses_lcu_event() {
        let event =
            json!({"uri": READY_CHECK, "data": {"state":"InProgress","playerResponse":"None"}});
        assert!(parse_ready_check_event(&event).unwrap().active);
    }

    #[test]
    fn ignores_unrelated_event() {
        let event = json!({"uri":"/lol-gameflow/v1/session", "data": {}});
        assert_eq!(
            parse_ready_check_event(&event),
            Err(ReadyCheckError::NotReadyCheckEvent)
        );
    }
}
