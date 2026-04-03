use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn label_ja(self) -> &'static str {
        match self {
            Self::Low => "低",
            Self::Medium => "中",
            Self::High => "高",
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailAction {
    Jump { label: String, target_id: String },
    Suggestion { replacement: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDetailComment {
    pub body: String,
    pub actions: Vec<DetailAction>,
}

pub fn parse_detail_comment_actions(input: &str) -> ParsedDetailComment {
    let mut body = String::new();
    let mut actions = Vec::new();
    let mut cursor = input;

    loop {
        let Some(start) = cursor.find("$[") else {
            body.push_str(cursor);
            break;
        };
        let (head, tail) = cursor.split_at(start);
        body.push_str(head);
        let Some(end_rel) = tail.find(']') else {
            body.push_str(tail);
            break;
        };

        let (directive, rest) = tail.split_at(end_rel + 1);
        let inner = &directive[2..directive.len() - 1];
        if let Some(action) = parse_directive(inner) {
            actions.push(action);
        } else {
            body.push_str(directive);
        }
        cursor = rest;
    }

    ParsedDetailComment {
        body: body.trim().to_string(),
        actions,
    }
}

fn parse_directive(input: &str) -> Option<DetailAction> {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("jump ") {
        let mut parts = rest.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            return None;
        }
        let target_id = parts.pop()?.to_string();
        let label = parts.join(" ");
        if label.is_empty() || target_id.is_empty() {
            return None;
        }
        return Some(DetailAction::Jump { label, target_id });
    }

    if let Some(rest) = trimmed.strip_prefix("suggestion ") {
        let replacement = rest.trim().replace("\\n", "\n");
        if replacement.is_empty() {
            return None;
        }
        return Some(DetailAction::Suggestion { replacement });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{DetailAction, Priority, ProofreadResult, parse_detail_comment_actions};

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

    #[test]
    fn japanese_label_is_stable() {
        assert_eq!(Priority::Low.label_ja(), "低");
        assert_eq!(Priority::Medium.label_ja(), "中");
        assert_eq!(Priority::High.label_ja(), "高");
    }

    #[test]
    fn parses_jump_directive() {
        let parsed = parse_detail_comment_actions(
            "ここを確認してください。$[jump このオブジェクト l10-f32-n10]",
        );
        assert_eq!(parsed.body, "ここを確認してください。");
        assert_eq!(
            parsed.actions,
            vec![DetailAction::Jump {
                label: "このオブジェクト".to_string(),
                target_id: "l10-f32-n10".to_string()
            }]
        );
    }

    #[test]
    fn parses_suggestion_directive() {
        let parsed = parse_detail_comment_actions(
            "改善案です。$[suggestion 置換後テキスト、\\n改行も入れられる]",
        );
        assert_eq!(parsed.body, "改善案です。");
        assert_eq!(
            parsed.actions,
            vec![DetailAction::Suggestion {
                replacement: "置換後テキスト、\n改行も入れられる".to_string()
            }]
        );
    }

    #[test]
    fn keeps_unknown_directive_in_body() {
        let parsed = parse_detail_comment_actions("x $[unknown y]");
        assert_eq!(parsed.body, "x $[unknown y]");
        assert!(parsed.actions.is_empty());
    }
}
