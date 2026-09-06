//! Immutable authority lifecycle objects. Existence is not admission.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::evidence::{AuthorityId, ContentObjectId};

macro_rules! identity {
  ($name:ident) => {
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
    #[serde(transparent)]
    pub struct $name(pub ContentObjectId);
  };
}
identity!(SpecSnapshotId);
identity!(ProposalId);
identity!(ReconciliationReportId);
identity!(ClarificationId);
identity!(AdmissionId);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpecSnapshot {
  pub schema_version: u32,
  pub path: String,
  pub content: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Authority {
  pub schema_version: u32,
  pub spec: SpecSnapshotId,
  pub contract: ContentObjectId,
  pub surface: ContentObjectId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Issue {
  pub code: String,
  pub message: String,
  pub blocking: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorityProposal {
  pub schema_version: u32,
  pub authority: AuthorityId,
  pub issues: Vec<Issue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Finding {
  pub code: String,
  pub message: String,
  pub blocking: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReconciliationReport {
  pub schema_version: u32,
  pub proposal: ProposalId,
  pub findings: Vec<Finding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Clarification {
  pub schema_version: u32,
  pub proposal: ProposalId,
  pub clarification: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Admission {
  pub schema_version: u32,
  pub proposal: ProposalId,
  pub reconciliation: ReconciliationReportId,
  pub authority: AuthorityId,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdmissionError {
  #[error("unsupported authority lifecycle object version")]
  UnsupportedVersion,
  #[error("admission proposal does not match exact proposal")]
  ProposalMismatch,
  #[error("reconciliation targets another proposal")]
  ReconciliationProposalMismatch,
  #[error("admission reconciliation does not match exact report")]
  ReconciliationMismatch,
  #[error("admission authority does not match exact proposed authority")]
  AuthorityMismatch,
  #[error("authority specification does not match exact snapshot")]
  SpecificationMismatch,
  #[error("blocking issues or reconciliation findings prevent admission")]
  BlockingFindings,
}
