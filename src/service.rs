use crate::client::{ClientError, OpenAiCompatClient};
use crate::config::Credentials;
use std::collections::HashMap;

use crate::marker::{CollectedTarget, ObjectLocation, TargetType};
use crate::prompt::{PromptError, build_prompt};
use crate::result::{ProofreadDetail, ProofreadResult};

#[derive(Debug)]
pub enum ProofreadServiceError {
    NoTextTargets,
    Prompt(PromptError),
    Client(ClientError),
    UnknownTarget(String),
}

impl std::fmt::Display for ProofreadServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTextTargets => write!(f, "no text targets found"),
            Self::Prompt(err) => write!(f, "{err}"),
            Self::Client(err) => write!(f, "{err}"),
            Self::UnknownTarget(id) => {
                write!(f, "proofreading result references unknown target: {id}")
            }
        }
    }
}

impl std::error::Error for ProofreadServiceError {}

impl From<ClientError> for ProofreadServiceError {
    fn from(value: ClientError) -> Self {
        Self::Client(value)
    }
}

impl From<PromptError> for ProofreadServiceError {
    fn from(value: PromptError) -> Self {
        Self::Prompt(value)
    }
}

pub struct ProofreadService;

impl ProofreadService {
    pub fn build_prompt(
        template_id: &str,
        project_info: &str,
        project_prompt: &str,
        targets: &[CollectedTarget],
    ) -> Result<String, ProofreadServiceError> {
        let text_targets: Vec<CollectedTarget> = targets
            .iter()
            .filter(|target| target.target_type == TargetType::Text)
            .cloned()
            .collect();
        if text_targets.is_empty() {
            return Err(ProofreadServiceError::NoTextTargets);
        }

        Ok(build_prompt(
            template_id,
            project_info,
            project_prompt,
            &text_targets,
        )?)
    }

    pub fn run(
        template_id: &str,
        project_info: &str,
        project_prompt: &str,
        targets: &[CollectedTarget],
        locations: &HashMap<String, ObjectLocation>,
        credentials: &Credentials,
    ) -> Result<ProofreadResult, ProofreadServiceError> {
        let prompt = Self::build_prompt(template_id, project_info, project_prompt, targets)?;
        let client = OpenAiCompatClient::new(
            credentials.base_url.clone(),
            credentials.model.clone(),
            credentials.api_key.clone(),
        );
        let raw_result = client.request_proofread(&prompt)?;
        let details = raw_result
            .details
            .into_iter()
            .map(|detail| {
                let location = locations
                    .get(&detail.id)
                    .ok_or_else(|| ProofreadServiceError::UnknownTarget(detail.id.clone()))?;
                Ok(ProofreadDetail {
                    object: location.object,
                    position: location.position,
                    priority: detail.priority,
                    comment: detail.comment,
                    resolved: false,
                })
            })
            .collect::<Result<Vec<_>, ProofreadServiceError>>()?;
        Ok(ProofreadResult {
            all: raw_result.all,
            details,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::config::Credentials;

    use super::{ProofreadService, ProofreadServiceError};

    #[test]
    fn returns_error_when_no_text_targets() {
        let result = ProofreadService::run(
            crate::prompt::BUILTIN_TEMPLATE_ID,
            "project",
            "prompt",
            &[],
            &HashMap::new(),
            &Credentials::default(),
        );
        assert!(matches!(result, Err(ProofreadServiceError::NoTextTargets)));
    }
}
