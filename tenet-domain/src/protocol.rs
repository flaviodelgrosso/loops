use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkflowPhase {
  SpecRequired,
  AuthorityRequired,
  AuthorityReconciliation,
  AuthorityClarification,
  AuthorityAdmission,
  AuthorityStale,
  Incompatible,
  Implementation,
  Completed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContextFacts {
  pub spec_exists: bool,
  pub compatible: bool,
  pub proposal_exists: bool,
  pub reconciliation_exists: bool,
  pub reconciliation_blocked: bool,
  pub admission_exists: bool,
  pub authority_current: bool,
  pub final_satisfied: bool,
  pub current_matches_final: bool,
}
