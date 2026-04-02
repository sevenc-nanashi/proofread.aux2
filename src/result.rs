use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofreadDetail {
    pub id: String,
    pub priority: Priority,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofreadResult {
    pub all: String,
    pub details: Vec<ProofreadDetail>,
}

impl ProofreadResult {
    pub fn from_json(input: &str) -> serde_json::Result<Self> {
        serde_json::from_str(input)
    }
}

#[cfg(test)]
mod tests {
    use super::{Priority, ProofreadResult};

    #[test]
    fn parses_valid_result_json() {
        let input = r#"{
            "all":"全体コメント",
            "details":[{"id":"t-1","priority":"high","comment":"修正してください"}]
        }"#;
        let result = ProofreadResult::from_json(input).expect("must parse");
        assert_eq!(result.all, "全体コメント");
        assert_eq!(result.details.len(), 1);
        assert_eq!(result.details[0].id, "t-1");
        assert_eq!(result.details[0].priority, Priority::High);
    }

    #[test]
    fn rejects_invalid_priority() {
        let input = r#"{
            "all":"x",
            "details":[{"id":"t-1","priority":"urgent","comment":"x"}]
        }"#;
        let parsed = ProofreadResult::from_json(input);
        assert!(parsed.is_err());
    }
}
