use serde::Deserialize;
use tenet_domain::{
  algebra::*,
  authority::*,
  completion::Verdict,
  contract::RequirementId,
  evidence::{
    AuthorityId, CandidateId, ContentObjectId, ExecutionEnvironmentIdentity, ExecutionProvenance,
    RunnerIdentity,
  },
};
use tenet_kernel::{
  algebra::{AdmissionChain, evaluate, validate_completion_admission, validate_contract},
  authority::{
    admission_id, authority_id, proposal_id, reconciliation_report_id, spec_snapshot_id,
  },
};

fn content(label: &str) -> ContentObjectId {
  ContentObjectId(format!("sha256:{label:0<64}"))
}

struct Fixture {
  spec: SpecSnapshot,
  authority: Authority,
  proposal: AuthorityProposal,
  report: ReconciliationReport,
  admission: Admission,
}

impl Fixture {
  fn new(contract: &CompletionContractV1) -> Self {
    let spec = SpecSnapshot {
      schema_version: 1,
      path: "SPEC.md".into(),
      content: b"spec".to_vec(),
    };
    let authority = Authority {
      schema_version: 1,
      spec: spec_snapshot_id(&spec).unwrap(),
      contract: ContentObjectId(tenet_kernel::digest::canonical_digest(contract).unwrap()),
      surface: content("surface"),
    };
    let proposal = AuthorityProposal {
      schema_version: 1,
      authority: authority_id(&authority).unwrap(),
      issues: vec![],
    };
    let report = ReconciliationReport {
      schema_version: 1,
      proposal: proposal_id(&proposal).unwrap(),
      findings: vec![],
    };
    let admission = Admission {
      schema_version: 1,
      proposal: proposal_id(&proposal).unwrap(),
      reconciliation: reconciliation_report_id(&report).unwrap(),
      authority: authority_id(&authority).unwrap(),
    };
    Self {
      spec,
      authority,
      proposal,
      report,
      admission,
    }
  }

  fn chain(&self) -> AdmissionChain<'_> {
    AdmissionChain {
      admission: &self.admission,
      proposal: &self.proposal,
      report: &self.report,
      authority: &self.authority,
      spec: &self.spec,
    }
  }
}

fn verifier(id: &str, material: VerifierMaterial) -> Verifier {
  Verifier {
    id: VerifierId(id.into()),
    material,
  }
}

fn criterion(
  id: &str,
  control: EvidenceControlRequirementV1,
  assurance: AssuranceRequirementV1,
  verifiers: Vec<Verifier>,
) -> Criterion {
  Criterion {
    id: CriterionId(id.into()),
    proposition: format!("{id} holds"),
    verifiers,
    evidence: EvidenceRequirementV1 { control, assurance },
  }
}

fn contract(criteria: Vec<Criterion>) -> CompletionContractV1 {
  CompletionContractV1 {
    schema_version: 1,
    policy: CompletionPolicyId(COMPLETION_POLICY_V1.into()),
    requirements: vec![Requirement {
      id: RequirementId("R1".into()),
      statement: "requirement one".into(),
      criteria,
    }],
  }
}

fn run(fixture: &Fixture, verifier: &str, profile: &str, exit_code: Option<i32>) -> VerifierRun {
  VerifierRun {
    authority: authority_id(&fixture.authority).unwrap(),
    candidate: CandidateId(content("candidate")),
    verifier: VerifierId(verifier.into()),
    observation: ExecutionObservation {
      result: match exit_code {
        Some(0) => EvidenceResult::Pass,
        Some(125 | 126) => EvidenceResult::Inconclusive,
        Some(_) => EvidenceResult::Fail,
        None => EvidenceResult::InfrastructureError,
      },
      exit_code,
      timed_out: false,
      infrastructure_error: None,
    },
    context: ExecutionContext {
      assurance: AssuranceProfileId(profile.into()),
      runner_semantics: RunnerSemanticsId(RUNNER_SEMANTICS_V1.into()),
      platform: PlatformInformation {
        os: "test-os".into(),
        architecture: "test-arch".into(),
      },
      resolved_program: Some("/test/verifier".into()),
      resolved_program_digest: Some("sha256:program".into()),
    },
    provenance: ExecutionProvenance {
      runner_identity: RunnerIdentity("test-runner".into()),
      runner_semantics: RunnerSemanticsId(RUNNER_SEMANTICS_V1.into()),
      assurance: AssuranceProfileId(profile.into()),
      tenet_version: "test".into(),
      platform: PlatformInformation {
        os: "test-os".into(),
        architecture: "test-arch".into(),
      },
      resolved_program: Some("/test/verifier".into()),
      resolved_program_digest: Some("sha256:program".into()),
      execution_environment_identity: ExecutionEnvironmentIdentity("test-environment".into()),
    },
  }
}

fn evaluation(fixture: &Fixture, scope: EvaluationScope, runs: Vec<VerifierRun>) -> Evaluation {
  Evaluation {
    admission: admission_id(&fixture.admission).unwrap(),
    authority: authority_id(&fixture.authority).unwrap(),
    candidate: CandidateId(content("candidate")),
    scope,
    runs,
  }
}

#[test]
fn candidate_controlled_verifier_cannot_satisfy_authority_bound_only() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::AuthorityBoundOnly,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  assert_eq!(
    validate_completion_admission(&contract, &fixture.chain()),
    Err(AlgebraError::ImpossibleEvidenceControl("C1".into()))
  );
}

#[test]
fn candidate_controlled_evidence_requires_explicit_permission() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  assert_eq!(validate_contract(&contract), Ok(()));
}

#[test]
fn one_authority_bound_verifier_satisfies_mixed_control_requirement() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::AtLeastOneAuthorityBound,
    AssuranceRequirementV1::LocalOrStronger,
    vec![
      verifier("V1", VerifierMaterial::Candidate),
      verifier("V2", VerifierMaterial::AuthorityBundle),
    ],
  )]);
  assert_eq!(validate_contract(&contract), Ok(()));
}

#[test]
fn candidate_only_set_cannot_satisfy_mixed_control_requirement() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::AtLeastOneAuthorityBound,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  assert_eq!(
    validate_completion_admission(&contract, &fixture.chain()),
    Err(AlgebraError::ImpossibleEvidenceControl("C1".into()))
  );
}

#[test]
fn verifier_cannot_be_shared_between_criteria() {
  let contract = contract(vec![
    criterion(
      "C1",
      EvidenceControlRequirementV1::CandidateControlledPermitted,
      AssuranceRequirementV1::LocalOrStronger,
      vec![verifier("V1", VerifierMaterial::Candidate)],
    ),
    criterion(
      "C2",
      EvidenceControlRequirementV1::CandidateControlledPermitted,
      AssuranceRequirementV1::LocalOrStronger,
      vec![verifier("V1", VerifierMaterial::Candidate)],
    ),
  ]);
  assert_eq!(
    validate_contract(&contract),
    Err(AlgebraError::DuplicateId {
      kind: "verifier",
      id: "V1".into(),
    })
  );
}

#[test]
fn blank_semantic_identifiers_cannot_authorize_completion() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier(" ", VerifierMaterial::Candidate)],
  )]);
  assert_eq!(
    validate_contract(&contract),
    Err(AlgebraError::BlankId { kind: "verifier" })
  );
}

#[test]
fn evaluation_scope_rejects_unknown_semantic_fields() {
  let value = serde_json::json!({
    "kind": "requirement",
    "requirement": "R1",
    "futureConstraint": "ignored"
  });
  assert!(serde_json::from_value::<EvaluationScope>(value).is_err());
}

#[test]
fn local_success_is_inadmissible_for_protected_criterion() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::Protected,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(
      &fixture,
      EvaluationScope::Final,
      vec![run(&fixture, "V1", LOCAL_V1, Some(0))],
    ),
  )
  .unwrap();
  assert_eq!(
    result.requirements[0].criteria[0].state,
    CriterionState::InadmissibleEvidence
  );
}

#[test]
fn run_for_another_requirement_is_outside_scope() {
  let mut contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  contract.requirements.push(Requirement {
    id: RequirementId("R2".into()),
    statement: "requirement two".into(),
    criteria: vec![criterion(
      "C2",
      EvidenceControlRequirementV1::CandidateControlledPermitted,
      AssuranceRequirementV1::LocalOrStronger,
      vec![verifier("V2", VerifierMaterial::Candidate)],
    )],
  });
  let fixture = Fixture::new(&contract);
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(
      &fixture,
      EvaluationScope::Requirement {
        requirement: RequirementId("R2".into()),
      },
      vec![run(&fixture, "V1", LOCAL_V1, Some(0))],
    ),
  );
  assert_eq!(result, Err(AlgebraError::VerifierOutsideScope("V1".into())));
}

#[test]
fn run_for_another_authority_is_rejected() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let mut foreign = run(&fixture, "V1", LOCAL_V1, Some(0));
  foreign.authority = AuthorityId(content("other-authority"));
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(&fixture, EvaluationScope::Final, vec![foreign]),
  );
  assert_eq!(result, Err(AlgebraError::RunAuthorityMismatch));
}

#[test]
fn run_for_another_candidate_is_rejected() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let mut foreign = run(&fixture, "V1", LOCAL_V1, Some(0));
  foreign.candidate = CandidateId(content("other-candidate"));
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(&fixture, EvaluationScope::Final, vec![foreign]),
  );
  assert_eq!(result, Err(AlgebraError::RunCandidateMismatch));
}

#[test]
fn missing_required_run_is_reported() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(&fixture, EvaluationScope::Final, vec![]),
  )
  .unwrap();
  assert_eq!(
    result.requirements[0].criteria[0].state,
    CriterionState::MissingEvidence
  );
}

#[test]
fn duplicate_run_is_rejected() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(
      &fixture,
      EvaluationScope::Final,
      vec![
        run(&fixture, "V1", LOCAL_V1, Some(0)),
        run(&fixture, "V1", LOCAL_V1, Some(0)),
      ],
    ),
  );
  assert_eq!(result, Err(AlgebraError::DuplicateRun("V1".into())));
}

#[test]
fn unrelated_extra_run_is_rejected() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(
      &fixture,
      EvaluationScope::Final,
      vec![run(&fixture, "V2", LOCAL_V1, Some(0))],
    ),
  );
  assert_eq!(result, Err(AlgebraError::VerifierOutsideScope("V2".into())));
}

#[test]
fn evaluation_validates_exact_admission_identity() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let mut evaluation = evaluation(&fixture, EvaluationScope::Final, vec![]);
  evaluation.admission = AdmissionId(content("other-decision"));
  assert_eq!(
    evaluate(&contract, &fixture.chain(), &evaluation),
    Err(AlgebraError::AdmissionMismatch)
  );
}

#[test]
fn evaluation_rejects_contract_substitution_under_valid_admission() {
  let admitted = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&admitted);
  let substituted = contract(vec![criterion(
    "C2",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V2", VerifierMaterial::Candidate)],
  )]);
  assert_eq!(
    evaluate(
      &substituted,
      &fixture.chain(),
      &evaluation(&fixture, EvaluationScope::Final, vec![]),
    ),
    Err(AlgebraError::ContractMismatch)
  );
}

#[test]
fn unknown_policy_and_assurance_profiles_fail_closed() {
  let mut contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  contract.policy = CompletionPolicyId("tenet:completion-policy:v99".into());
  assert_eq!(
    validate_completion_admission(&contract, &fixture.chain()),
    Err(AlgebraError::UnsupportedCompletionPolicy(
      "tenet:completion-policy:v99".into()
    ))
  );

  contract.policy = CompletionPolicyId(COMPLETION_POLICY_V1.into());
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(
      &fixture,
      EvaluationScope::Final,
      vec![run(&fixture, "V1", "FUTURE_V9", Some(0))],
    ),
  );
  assert_eq!(
    result,
    Err(AlgebraError::UnsupportedAssuranceProfile(
      "FUTURE_V9".into()
    ))
  );
}

#[test]
fn unknown_runner_semantics_fail_closed() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let mut verifier_run = run(&fixture, "V1", LOCAL_V1, Some(0));
  verifier_run.context.runner_semantics = RunnerSemanticsId("tenet:runner-semantics:v99".into());
  assert_eq!(
    evaluate(
      &contract,
      &fixture.chain(),
      &evaluation(&fixture, EvaluationScope::Final, vec![verifier_run]),
    ),
    Err(AlgebraError::UnsupportedRunnerSemantics(
      "tenet:runner-semantics:v99".into()
    ))
  );
}

#[test]
fn all_verifiers_are_required_for_a_criterion() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::LocalOrStronger,
    vec![
      verifier("V1", VerifierMaterial::Candidate),
      verifier("V2", VerifierMaterial::Candidate),
    ],
  )]);
  let fixture = Fixture::new(&contract);
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(
      &fixture,
      EvaluationScope::Final,
      vec![run(&fixture, "V1", LOCAL_V1, Some(0))],
    ),
  )
  .unwrap();
  assert_eq!(
    result.requirements[0].criteria[0].state,
    CriterionState::MissingEvidence
  );
}

#[test]
fn timeout_overrides_a_recognized_failure_exit() {
  let contract = contract(vec![criterion(
    "C1",
    EvidenceControlRequirementV1::CandidateControlledPermitted,
    AssuranceRequirementV1::Protected,
    vec![verifier("V1", VerifierMaterial::Candidate)],
  )]);
  let fixture = Fixture::new(&contract);
  let mut failed = run(&fixture, "V1", LOCAL_V1, Some(1));
  failed.observation.timed_out = true;
  let result = evaluate(
    &contract,
    &fixture.chain(),
    &evaluation(&fixture, EvaluationScope::Final, vec![failed]),
  )
  .unwrap();
  assert_eq!(result.verdict, Some(Verdict::InfrastructureError));
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoldenVector {
  name: String,
  assurance_requirement: AssuranceRequirementV1,
  profile: String,
  exit_code: Option<i32>,
  timed_out: bool,
  expected_state: CriterionState,
  expected_verdict: Verdict,
}

#[test]
fn completion_policy_v1_matches_golden_vectors() {
  let vectors: Vec<GoldenVector> =
    serde_json::from_str(include_str!("fixtures/completion_policy_v1.json")).unwrap();
  for vector in vectors {
    let contract = contract(vec![criterion(
      "C1",
      EvidenceControlRequirementV1::CandidateControlledPermitted,
      vector.assurance_requirement,
      vec![verifier("V1", VerifierMaterial::Candidate)],
    )]);
    let fixture = Fixture::new(&contract);
    let mut verifier_run = run(&fixture, "V1", &vector.profile, vector.exit_code);
    verifier_run.observation.timed_out = vector.timed_out;
    let result = evaluate(
      &contract,
      &fixture.chain(),
      &evaluation(&fixture, EvaluationScope::Final, vec![verifier_run]),
    )
    .unwrap_or_else(|error| panic!("golden vector `{}` failed: {error}", vector.name));
    assert_eq!(
      result.requirements[0].criteria[0].state, vector.expected_state,
      "{}",
      vector.name
    );
    assert_eq!(
      result.verdict,
      Some(vector.expected_verdict),
      "{}",
      vector.name
    );
  }
}
