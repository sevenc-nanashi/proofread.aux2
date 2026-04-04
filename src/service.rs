use crate::client::{ClientError, OpenAiCompatClient};
use crate::config::Credentials;
use crate::marker::{CollectedTarget, TargetType};
use crate::prompt::{PromptError, build_prompt};
use crate::result::ProofreadResult;

#[derive(Debug)]
pub enum ProofreadServiceError {
    NoTextTargets,
    Prompt(PromptError),
    Client(ClientError),
}

impl std::fmt::Display for ProofreadServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoTextTargets => write!(f, "no text targets found"),
            Self::Prompt(err) => write!(f, "{err}"),
            Self::Client(err) => write!(f, "{err}"),
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
    pub fn run(
        template_id: &str,
        project_info: &str,
        project_prompt: &str,
        targets: &[CollectedTarget],
        credentials: &Credentials,
    ) -> Result<ProofreadResult, ProofreadServiceError> {
        let text_targets: Vec<CollectedTarget> = targets
            .iter()
            .filter(|target| target.target_type == TargetType::Text)
            .cloned()
            .collect();
        if text_targets.is_empty() {
            return Err(ProofreadServiceError::NoTextTargets);
        }

        let prompt = build_prompt(template_id, project_info, project_prompt, &text_targets)?;
        let client = OpenAiCompatClient::new(
            credentials.base_url.clone(),
            credentials.model.clone(),
            credentials.api_key.clone(),
        );
        let result = client.request_proofread(&prompt)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Credentials;

    use super::{ProofreadService, ProofreadServiceError};

    #[test]
    fn returns_error_when_no_text_targets() {
        let result = ProofreadService::run(
            crate::prompt::BUILTIN_TEMPLATE_ID,
            "project",
            "prompt",
            &[],
            &Credentials::default(),
        );
        assert!(matches!(result, Err(ProofreadServiceError::NoTextTargets)));
    }
}
