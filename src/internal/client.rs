//! Internal client implementation

use futures::stream::StreamExt;

use crate::errors::Result;
use crate::types::config::ClaudeAgentOptions;
use crate::types::messages::Message;

use super::message_parser::MessageParser;
use super::transport::subprocess::QueryPrompt;
use super::transport::{SubprocessTransport, Transport};

/// Internal client for processing queries
pub struct InternalClient {
    transport: SubprocessTransport,
}

impl InternalClient {
    /// Create a new client
    pub fn new(prompt: QueryPrompt, options: ClaudeAgentOptions) -> Result<Self> {
        let transport = SubprocessTransport::new(prompt, options)?;
        Ok(Self { transport })
    }

    /// Connect and get messages
    pub async fn execute(self) -> Result<Vec<Message>> {
        // Connect
        self.transport.connect().await?;

        // Collect all messages
        let mut messages = Vec::new();
        {
            let mut stream = self.transport.read_messages();

            while let Some(result) = stream.next().await {
                let json = result?;
                match MessageParser::parse(json) {
                    Ok(message) => messages.push(message),
                    Err(e) if e.is_unknown_message_type() => {
                        eprintln!("Warning: {}", e);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
            // Stream is dropped here
        }

        // Close transport
        self.transport.close().await?;

        Ok(messages)
    }
}
