use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
  authority::AdmissionId,
  evidence::{AuthorityId, CandidateId, ContentObjectId, ExecutionProvenance},
};

macro_rules! string_id {
  ($name:ident) => {
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
    #[serde(transparent)]
    pub struct $name(pub String);
  };
}

string_id!(CriterionId);
string_id!(VerifierId);
string_id!(CompletionPolicyId);
string_id!(AssuranceProfileId);
string_id!(RunnerSemanticsId);
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct EvaluationId(pub ContentObjectId);

pub const COMPLETION_POLICY_V1: &str = "tenet:completion-policy:v1";
pub const RUNNER_SEMANTICS_V1: &str = "tenet:runner-semantics:v1";
pub const LOCAL_V1: &str = "LOCAL_V1";
pub const PROTECTED_V1: &str = "PROTECTED_V1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceControl {
  CandidateControlled,
  AuthorityBound,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VerifierMaterial {
  Candidate,
  AuthorityBundle,
}

impl VerifierMaterial {
  pub const fn control(self) -> EvidenceControl {
    match self {
      Self::Candidate => EvidenceControl::CandidateControlled,
      Self::AuthorityBundle => EvidenceControl::AuthorityBound,
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceControlRequirementV1 {
  AuthorityBoundOnly,
  AtLeastOneAuthorityBound,
  CandidateControlledPermitted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssuranceRequirementV1 {
  LocalOrStronger,
  Protected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceRequirementV1 {
  pub control: EvidenceControlRequirementV1,
  pub assurance: AssuranceRequirementV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Verifier {
  pub id: VerifierId,
  pub material: VerifierMaterial,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Criterion {
  pub id: CriterionId,
  pub proposition: String,
  #[schemars(length(min = 1))]
  pub verifiers: Vec<Verifier>,
  pub evidence: EvidenceRequirementV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Requirement {
  pub id: crate::contract::RequirementId,
  pub statement: String,
  #[schemars(length(min = 1))]
  pub criteria: Vec<Criterion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionContractV1 {
  pub schema_version: u32,
  pub policy: CompletionPolicyId,
  #[schemars(length(min = 1))]
  pub requirements: Vec<Requirement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionObservation {
  pub result: EvidenceResult,
  pub exit_code: Option<i32>,
  pub timed_out: bool,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub infrastructure_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformInformation {
  pub os: String,
  pub architecture: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionContext {
  pub assurance: AssuranceProfileId,
  pub runner_semantics: RunnerSemanticsId,
  pub platform: PlatformInformation,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub resolved_program: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub resolved_program_digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifierRun {
  pub authority: AuthorityId,
  pub candidate: CandidateId,
  pub verifier: VerifierId,
  pub observation: ExecutionObservation,
  pub context: ExecutionContext,
  pub provenance: ExecutionProvenance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
  tag = "kind",
  rename_all = "snake_case",
  rename_all_fields = "camelCase",
  deny_unknown_fields
)]
pub enum EvaluationScope {
  Requirement {
    requirement: crate::contract::RequirementId,
  },
  Final,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evaluation {
  pub admission: AdmissionId,
  pub authority: AuthorityId,
  pub candidate: CandidateId,
  pub scope: EvaluationScope,
  pub runs: Vec<VerifierRun>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceResult {
  Pass,
  Fail,
  Inconclusive,
  InfrastructureError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CriterionState {
  Satisfied,
  Contradicted,
  MissingEvidence,
  InadmissibleEvidence,
  Inconclusive,
  InfrastructureError,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompletionState {
  Satisfied,
  Contradicted,
  Inconclusive,
  InfrastructureError,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CriterionEvaluation {
  pub criterion: CriterionId,
  pub state: CriterionState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequirementEvaluation {
  pub requirement: crate::contract::RequirementId,
  pub state: CompletionState,
  pub criteria: Vec<CriterionEvaluation>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionEvaluation {
  pub state: CompletionState,
  pub requirements: Vec<RequirementEvaluation>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub verdict: Option<crate::completion::Verdict>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AlgebraError {
  #[error("unsupported completion contract schema version {0}")]
  UnsupportedSchemaVersion(u32),
  #[error("unsupported completion policy `{0}`")]
  UnsupportedCompletionPolicy(String),
  #[error("unsupported assurance profile `{0}`")]
  UnsupportedAssuranceProfile(String),
  #[error("unsupported runner semantics `{0}`")]
  UnsupportedRunnerSemantics(String),
  #[error("completion contract contains no requirements")]
  MissingRequirements,
  #[error("requirement `{0}` contains no criteria")]
  MissingCriteria(String),
  #[error("criterion `{0}` contains no verifiers")]
  MissingVerifiers(String),
  #[error("{kind} identifier must not be blank")]
  BlankId { kind: &'static str },
  #[error("{kind} statement must not be blank for `{id}`")]
  BlankStatement { kind: &'static str, id: String },
  #[error("duplicate {kind} identifier `{id}`")]
  DuplicateId { kind: &'static str, id: String },
  #[error("criterion `{0}` has an impossible evidence-control requirement")]
  ImpossibleEvidenceControl(String),
  #[error("evaluation references unknown requirement `{0}`")]
  UnknownRequirement(String),
  #[error("completion contract does not match the admitted authority")]
  ContractMismatch,
  #[error("evaluation admission does not match the admitted decision")]
  AdmissionMismatch,
  #[error("evaluation authority does not match the admitted authority")]
  AuthorityMismatch,
  #[error("verifier run is bound to the wrong authority")]
  RunAuthorityMismatch,
  #[error("verifier run is bound to the wrong candidate")]
  RunCandidateMismatch,
  #[error("verifier `{0}` is outside the evaluation scope")]
  VerifierOutsideScope(String),
  #[error("verifier `{0}` has more than one run")]
  DuplicateRun(String),
  #[error(transparent)]
  Admission(#[from] crate::authority::AdmissionError),
}
