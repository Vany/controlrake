use crate::address::Address;
use std::fmt::{Display, Formatter, Result as FmtResult};
use thiserror::Error;
use uuid::Uuid;

/// 📬 Raw message format: "target payload"
#[derive(Debug, Clone)]
pub struct RawMessage {
    pub raw: String,
}

/// 📨 Parsed message with extracted fields
#[derive(Debug, Clone)]
pub struct Message {
    pub target: Address,
    pub source: Option<Address>,
    pub payload: serde_json::Value,
    pub id: Option<MessageId>,    // For request-response
    pub reply: Option<MessageId>, // For responses
    pub finish: bool,             // End of response stream
}

/// 🆔 Unique message ID for request-response tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageId(pub String);

/// ❌ Message parsing errors
#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Message must contain space separator between target and payload")]
    MissingSpace,

    #[error("Address parsing failed: {0}")]
    AddressError(#[from] crate::address::AddressError),

    #[error("Invalid JSON payload: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
}

impl MessageId {
    /// 🆔 Generate new unique message ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// 📄 Get the inner ID string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

impl RawMessage {
    /// 📥 Create new raw message
    pub fn new(raw: String) -> Self {
        Self { raw }
    }

    /// ⚙️ Parse raw message into structured Message
    pub fn parse(&self) -> Result<Message, MessageError> {
        // Find first space separator
        let space_pos = self.raw.find(' ').ok_or(MessageError::MissingSpace)?;

        let target_str = &self.raw[..space_pos];
        let payload_str = &self.raw[space_pos + 1..];

        // Parse target address
        let target = target_str.parse::<Address>()?;

        // Parse JSON payload
        let mut payload: serde_json::Value = serde_json::from_str(payload_str)?;

        // Extract optional fields from payload
        let source = if let Some(from_val) = payload.get("from") {
            if let Some(from_str) = from_val.as_str() {
                // Only try to parse as Address if it looks like one (contains colon)
                if from_str.contains(':') {
                    match from_str.parse::<Address>() {
                        Ok(addr) => Some(addr),
                        Err(e) => {
                            tracing::warn!(
                                "❌ Failed to parse source address '{}': {}",
                                from_str,
                                e
                            );
                            None
                        }
                    }
                } else {
                    // For simple identifiers like "browser", skip source parsing
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let id = if let Some(id_val) = payload.get("id") {
            id_val.as_str().map(|id_str| MessageId(id_str.to_string()))
        } else {
            None
        };

        let reply = if let Some(reply_val) = payload.get("reply") {
            reply_val
                .as_str()
                .map(|reply_str| MessageId(reply_str.to_string()))
        } else {
            None
        };

        let finish = payload
            .get("finish")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        // Check if the payload was wrapped and unwrap it
        let final_payload = if payload
            .get("_wrapped")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            // Unwrap the original payload from the "data" field
            payload
                .get("data")
                .cloned()
                .unwrap_or(serde_json::Value::Null)
        } else {
            // Remove extracted fields from payload to keep it clean
            if let serde_json::Value::Object(ref mut map) = payload {
                map.remove("from");
                map.remove("id");
                map.remove("reply");
                map.remove("finish");
            }
            payload
        };

        Ok(Message {
            target,
            source,
            payload: final_payload,
            id,
            reply,
            finish,
        })
    }
}

impl Message {
    /// 📤 Create new message
    pub fn new(target: Address, payload: serde_json::Value) -> Self {
        Self {
            target,
            source: None,
            payload,
            id: None,
            reply: None,
            finish: false,
        }
    }

    /// 🏷️ Set source address
    pub fn with_source(mut self, source: Address) -> Self {
        self.source = Some(source);
        self
    }

    /// 🆔 Set message ID for request tracking
    pub fn with_id(mut self, id: MessageId) -> Self {
        self.id = Some(id);
        self
    }

    /// 🔄 Set reply ID for response tracking
    pub fn with_reply(mut self, reply: MessageId) -> Self {
        self.reply = Some(reply);
        self
    }

    /// 🏁 Mark message as final in response stream
    pub fn with_finish(mut self, finish: bool) -> Self {
        self.finish = finish;
        self
    }

    /// 📄 Convert message back to raw format
    pub fn to_raw(&self) -> String {
        let mut payload = self.payload.clone();

        // Always ensure we have an object to add metadata fields to
        let mut wrapper_needed = false;
        let mut final_payload = match payload {
            serde_json::Value::Object(ref mut _map) => {
                // Already an object, can add fields directly
                payload
            }
            _ => {
                // Not an object (array, string, number, etc.)
                // Wrap in an object with the original payload as "data"
                wrapper_needed = true;
                let mut wrapper = serde_json::Map::new();
                wrapper.insert("data".to_string(), payload);
                serde_json::Value::Object(wrapper)
            }
        };

        // Add metadata fields to the object
        if let serde_json::Value::Object(ref mut map) = final_payload {
            if let Some(ref source) = self.source {
                map.insert(
                    "from".to_string(),
                    serde_json::Value::String(source.to_string()),
                );
            }

            if let Some(ref id) = self.id {
                map.insert("id".to_string(), serde_json::Value::String(id.to_string()));
            }

            if let Some(ref reply) = self.reply {
                map.insert(
                    "reply".to_string(),
                    serde_json::Value::String(reply.to_string()),
                );
            }

            if self.finish {
                map.insert("finish".to_string(), serde_json::Value::Bool(true));
            }

            // Mark if we wrapped the original payload
            if wrapper_needed {
                map.insert("_wrapped".to_string(), serde_json::Value::Bool(true));
            }
        }

        format!("{} {}", self.target, final_payload)
    }
}

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_raw())
    }
}

impl From<String> for RawMessage {
    fn from(raw: String) -> Self {
        Self::new(raw)
    }
}

impl From<&str> for RawMessage {
    fn from(raw: &str) -> Self {
        Self::new(raw.to_string())
    }
}


