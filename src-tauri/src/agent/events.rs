use serde::{Deserialize, Serialize};

use super::types::{AgentReference, AgentUserInputRequest};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum AgentEvent {
    AgentStart {
        session_id: String,
    },
    TurnStart {
        mode: String,
    },
    ToolStart {
        tool: String,
        input: Option<String>,
    },
    ToolEnd {
        tool: String,
        output: Option<String>,
    },
    ReferenceAdded {
        reference: AgentReference,
    },
    FileChanged {
        path: String,
        tool: String,
        #[serde(rename = "existedBefore")]
        existed_before: bool,
        #[serde(rename = "previousContent", skip_serializing_if = "Option::is_none")]
        previous_content: Option<String>,
    },
    MessageDelta {
        text: String,
    },
    Error {
        message: String,
    },
    UserInputRequired {
        request: AgentUserInputRequest,
    },
    /// The agent starts its turn `used` of `max` (4.1).
    Step {
        used: usize,
        max: usize,
    },
    /// The model proposed a change of the contract: the card waits for a
    /// person (D-498).
    OperationProposed {
        card: Box<crate::operations::Card>,
    },
    Done {
        session_id: String,
    },
}

impl AgentEvent {
    pub fn tool_start(tool: impl Into<String>, input: Option<String>) -> Self {
        Self::ToolStart {
            tool: tool.into(),
            input,
        }
    }

    pub fn tool_end(tool: impl Into<String>, output: Option<String>) -> Self {
        Self::ToolEnd {
            tool: tool.into(),
            output,
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_event_serializes_with_camelcase_tag() {
        let value = serde_json::to_value(AgentEvent::ToolStart {
            tool: "base.search".to_string(),
            input: Some("query".to_string()),
        })
        .unwrap();

        assert_eq!(value["type"], "toolStart");
        assert_eq!(value["tool"], "base.search");
        assert_eq!(value["input"], "query");
    }

    #[test]
    fn file_changed_event_carries_bounded_rollback_metadata() {
        let value = serde_json::to_value(AgentEvent::FileChanged {
            path: "agent-workspace/report.md".to_string(),
            tool: "workspace.write_file".to_string(),
            existed_before: true,
            previous_content: Some("before".to_string()),
        })
        .unwrap();

        assert_eq!(value["type"], "fileChanged");
        assert_eq!(value["existedBefore"], true);
        assert_eq!(value["previousContent"], "before");
    }
}
