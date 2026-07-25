use serde::{Deserialize, Serialize};

use aviutl2::generic::{ObjectHandle, ObjectLayerFrame};

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

#[derive(Debug, Clone)]
pub struct ProofreadDetail {
    pub object: ObjectHandle,
    pub position: ObjectLayerFrame,
    pub priority: Priority,
    pub comment: String,
    pub resolved: bool,
}

#[derive(Debug, Clone)]
pub struct ProofreadResult {
    pub all: String,
    pub details: Vec<ProofreadDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct RawProofreadDetail {
    pub id: String,
    pub priority: Priority,
    pub comment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct RawProofreadResult {
    pub all: String,
    pub details: Vec<RawProofreadDetail>,
}

impl RawProofreadResult {
    pub fn from_json(input: &str) -> serde_json::Result<Self> {
        serde_json::from_str(input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailAction {
    Jump { target_id: String },
    Suggestion { replacement: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDetailComment {
    pub parts: Vec<CommentPart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentPart {
    Text(String),
    Action { label: String, action: DetailAction },
}

pub fn parse_detail_comment_actions(input: &str) -> ParsedDetailComment {
    let mut parts = Vec::new();
    let mut cursor = input;

    loop {
        let Some(start) = cursor.find("$[") else {
            if !cursor.is_empty() {
                parts.push(CommentPart::Text(cursor.to_string()));
            }
            break;
        };
        let (head, tail) = cursor.split_at(start);
        if !head.is_empty() {
            parts.push(CommentPart::Text(head.to_string()));
        }

        let Some(end_rel) = tail.find(']') else {
            parts.push(CommentPart::Text(tail.to_string()));
            continue;
        };
        let directive = &tail[2..end_rel];

        if let Some((label, action)) = parse_directive(directive) {
            parts.push(CommentPart::Action { label, action });
            cursor = &tail[(end_rel + 1)..];
        } else {
            let raw = &tail[..(end_rel + 1)];
            parts.push(CommentPart::Text(raw.to_string()));
            cursor = &tail[(end_rel + 1)..];
        }
    }

    ParsedDetailComment { parts }
}

fn parse_directive(input: &str) -> Option<(String, DetailAction)> {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("jump ") {
        let mut tokens = rest.split_whitespace().collect::<Vec<_>>();
        if tokens.len() < 2 {
            return None;
        }
        let target_id = tokens.pop()?.to_string();
        let label = tokens.join(" ");
        if label.is_empty() || target_id.is_empty() {
            return None;
        }
        return Some((label, DetailAction::Jump { target_id }));
    }

    if let Some(rest) = trimmed.strip_prefix("suggestion ") {
        let replacement = rest.trim().replace("\\n", "\n");
        if replacement.is_empty() {
            return None;
        }
        return Some((
            replacement.clone(),
            DetailAction::Suggestion { replacement },
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        CommentPart, DetailAction, Priority, RawProofreadResult, parse_detail_comment_actions,
    };

    #[test]
    fn parses_valid_result_json() {
        let input = r#"{
            "all":"全体コメント",
            "details":[{"id":"t-1","priority":"high","comment":"修正してください"}]
        }"#;
        let result = RawProofreadResult::from_json(input).expect("must parse");
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
        let parsed = RawProofreadResult::from_json(input);
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
            "位置は$[jump このオブジェクト l10-f32-n10]を確認してください。",
        );
        assert_eq!(
            parsed.parts,
            vec![
                CommentPart::Text("位置は".to_string()),
                CommentPart::Action {
                    label: "このオブジェクト".to_string(),
                    action: DetailAction::Jump {
                        target_id: "l10-f32-n10".to_string()
                    }
                },
                CommentPart::Text("を確認してください。".to_string())
            ]
        );
    }

    #[test]
    fn parses_suggestion_directive() {
        let parsed = parse_detail_comment_actions(
            "改善案として$[suggestion 置換後テキスト、\\n改行も入れられる]を適用できます。",
        );
        assert_eq!(
            parsed.parts,
            vec![
                CommentPart::Text("改善案として".to_string()),
                CommentPart::Action {
                    label: "置換後テキスト、\n改行も入れられる".to_string(),
                    action: DetailAction::Suggestion {
                        replacement: "置換後テキスト、\n改行も入れられる".to_string()
                    }
                },
                CommentPart::Text("を適用できます。".to_string())
            ]
        );
    }

    #[test]
    fn keeps_unknown_directive_as_text() {
        let parsed = parse_detail_comment_actions("x $[unknown z]");
        assert_eq!(
            parsed.parts,
            vec![
                CommentPart::Text("x ".to_string()),
                CommentPart::Text("$[unknown z]".to_string())
            ]
        );
    }
}
