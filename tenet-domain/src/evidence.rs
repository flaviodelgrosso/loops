use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::algebra::{AssuranceProfileId, PlatformInformation, RunnerSemanticsId};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String", into = "String")]
pub struct ContentObjectId(pub String);

impl TryFrom<String> for ContentObjectId {
  type Error = String;

  fn try_from(value: String) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}

impl From<ContentObjectId> for String {
  fn from(value: ContentObjectId) -> Self {
    value.0
  }
}

impl ContentObjectId {
  pub fn new(value: String) -> Result<Self, String> {
    let Some(digest) = value.strip_prefix("sha256:") else {
      return Err("content object ID must use sha256:<hex> form".into());
    };
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
      return Err(
        "content object ID must contain a 64-character hexadecimal SHA-256 digest".into(),
      );
    }
    Ok(Self(format!("sha256:{}", digest.to_ascii_lowercase())))
  }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AuthorityId(pub ContentObjectId);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct CandidateId(pub ContentObjectId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
  tag = "authority",
  rename_all = "snake_case",
  rename_all_fields = "camelCase",
  deny_unknown_fields
)]
pub enum OracleIdentity {
  Project {
    verifier_id: String,
    candidate_id: CandidateId,
    definition_digest: String,
  },
  AuthoritySnapshot {
    verifier_id: String,
    authority_id: AuthorityId,
    bundle_path: String,
    bundle_content_id: ContentObjectId,
    executable_content_id: ContentObjectId,
    definition_digest: String,
  },
  Unavailable {
    verifier_id: String,
    definition_digest: String,
  },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct RunnerIdentity(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ExecutionEnvironmentIdentity(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionProvenance {
  pub runner_identity: RunnerIdentity,
  pub runner_semantics: RunnerSemanticsId,
  pub assurance: AssuranceProfileId,
  pub tenet_version: String,
  pub platform: PlatformInformation,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub resolved_program: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub resolved_program_digest: Option<String>,
  pub oracle_identity: OracleIdentity,
  pub execution_environment_identity: ExecutionEnvironmentIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifierObservation {
  pub exit_code: Option<i32>,
  pub stdout: String,
  pub stderr: String,
  pub timed_out: bool,
}
