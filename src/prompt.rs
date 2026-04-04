use std::path::{Path, PathBuf};

use crate::marker::{CollectedTarget, TargetType};
use handlebars::Handlebars;
use serde::Serialize;

pub const BUILTIN_TEMPLATE_ID: &str = "__builtin_default__";
pub const BUILTIN_TEMPLATE_LABEL: &str = "組み込み: default.hbs";
const BUILTIN_TEMPLATE_SOURCE: &str = include_str!("prompts/default.hbs");

#[derive(Debug)]
pub enum PromptError {
    Io(std::io::Error),
    Template(handlebars::TemplateError),
    Render(handlebars::RenderError),
    InvalidTemplateId(String),
}

impl std::fmt::Display for PromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to access prompts directory: {err}"),
            Self::Template(err) => write!(f, "failed to parse prompt template: {err}"),
            Self::Render(err) => write!(f, "failed to render prompt template: {err}"),
            Self::InvalidTemplateId(id) => write!(f, "invalid prompt template id: {id}"),
        }
    }
}

impl std::error::Error for PromptError {}

impl From<std::io::Error> for PromptError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<handlebars::TemplateError> for PromptError {
    fn from(value: handlebars::TemplateError) -> Self {
        Self::Template(value)
    }
}

impl From<handlebars::RenderError> for PromptError {
    fn from(value: handlebars::RenderError) -> Self {
        Self::Render(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTemplate {
    pub id: String,
    pub label: String,
}

pub fn initialize_prompts_dir() -> Result<PathBuf, PromptError> {
    let dir = prompts_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn list_prompt_templates() -> Result<Vec<PromptTemplate>, PromptError> {
    let dir = initialize_prompts_dir()?;
    let mut templates = vec![PromptTemplate {
        id: BUILTIN_TEMPLATE_ID.to_string(),
        label: BUILTIN_TEMPLATE_LABEL.to_string(),
    }];

    let mut external = std::fs::read_dir(&dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            if path.extension().and_then(|v| v.to_str()) != Some("hbs") {
                return None;
            }
            let name = path.file_name()?.to_str()?.to_string();
            Some(PromptTemplate {
                id: name.clone(),
                label: format!("外部: {name}"),
            })
        })
        .collect::<Vec<_>>();
    external.sort_by(|a, b| a.id.cmp(&b.id));
    templates.extend(external);

    Ok(templates)
}

pub fn build_prompt(
    template_id: &str,
    project_info: &str,
    project_prompt: &str,
    targets: &[CollectedTarget],
) -> Result<String, PromptError> {
    let mut handlebars = Handlebars::new();
    let template_source = load_template_source(template_id)?;
    handlebars.register_template_string("proofread", template_source)?;
    let context = PromptContext {
        project_info,
        project_prompt,
        targets: targets
            .iter()
            .map(|target| PromptTarget {
                id: &target.id,
                target_type: match target.target_type {
                    TargetType::Text => "テキスト",
                },
                layer_name: &target.layer_name,
                start_time: &target.start_time,
                content: &target.content,
                color: target.color.as_deref(),
                memo: target.memo.as_deref().unwrap_or("(なし)"),
            })
            .collect(),
    };
    Ok(handlebars.render("proofread", &context)?)
}

fn load_template_source(template_id: &str) -> Result<String, PromptError> {
    if template_id == BUILTIN_TEMPLATE_ID {
        return Ok(BUILTIN_TEMPLATE_SOURCE.to_string());
    }

    let file_name = Path::new(template_id)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PromptError::InvalidTemplateId(template_id.to_string()))?;
    if file_name != template_id {
        return Err(PromptError::InvalidTemplateId(template_id.to_string()));
    }

    let path = initialize_prompts_dir()?.join(file_name);
    Ok(std::fs::read_to_string(path)?)
}

fn prompts_dir() -> Result<PathBuf, PromptError> {
    if let Some(path) =
        process_path::get_dylib_path().and_then(|v| v.parent().map(|parent| parent.to_path_buf()))
    {
        return Ok(path.join("prompts"));
    }

    let exe_path = std::env::current_exe().map_err(PromptError::Io)?;
    let base = exe_path
        .parent()
        .ok_or_else(|| std::io::Error::other("failed to resolve executable directory"))?;
    Ok(base.join("prompts"))
}

#[derive(Debug, Serialize)]
struct PromptContext<'a> {
    project_info: &'a str,
    project_prompt: &'a str,
    targets: Vec<PromptTarget<'a>>,
}

#[derive(Debug, Serialize)]
struct PromptTarget<'a> {
    id: &'a str,
    target_type: &'a str,
    layer_name: &'a str,
    start_time: &'a str,
    content: &'a str,
    color: Option<&'a str>,
    memo: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::marker::CollectedTarget;

    use super::{BUILTIN_TEMPLATE_ID, build_prompt};

    #[test]
    fn prompt_contains_required_sections() {
        let targets = vec![CollectedTarget::text(
            "t-1",
            "Layer 1",
            "00:00:01",
            "こんにちは",
            Some("#ffffff".into()),
        )];

        let prompt = build_prompt(BUILTIN_TEMPLATE_ID, "project", "視聴者は中学生", &targets)
            .expect("must render");
        assert!(prompt.contains("# 注意"));
        assert!(prompt.contains("# プロジェクトに関する特記事項"));
        assert!(prompt.contains("視聴者は中学生"));
        assert!(prompt.contains("## t-1"));
        assert!(prompt.contains("色：#ffffff"));
        assert!(prompt.contains("$[jump ボタン表示名 対象ID]"));
        assert!(prompt.contains("$[suggestion 置換後テキスト]"));
    }
}
