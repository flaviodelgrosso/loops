use std::{
  fs,
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
};

use tenet_application::{
  application::{AuthoritySubmitRequest, InitializeRequest, RequirementCheckRequest, Tenet},
  ports::{ExecutedVerifier, Repository, VerifierRun, VerifierRunner},
  response::AuthoritySubmissionResult,
};
use tenet_domain::{
  algebra::{
    AssuranceProfileId, AssuranceRequirementV1, COMPLETION_POLICY_V1, CompletionContractV1,
    CompletionPolicyId, CompletionState, Criterion, CriterionId, Evaluation, EvaluationScope,
    EvidenceControlRequirementV1, EvidenceRequirementV1, EvidenceResult, LOCAL_V1,
    PlatformInformation, RUNNER_SEMANTICS_V1, Requirement, RunnerSemanticsId, Verifier, VerifierId,
    VerifierMaterial,
  },
  authority::AdmissionId,
  completion::Verdict,
  contract::RequirementId,
  evidence::{
    AuthorityId, ExecutionEnvironmentIdentity, ExecutionProvenance, RunnerIdentity,
    VerifierObservation,
  },
  policy::{
    CandidateCapturePolicy, CommandArgument, CommandCwd, CommandSpec, EnvironmentSpec,
    ProjectConfig, VerifierAuthority, VerifierSpec,
  },
  protocol::WorkflowPhase,
  snapshot::{SnapshotSemanticsId, TreeManifest},
};
use tenet_workspace::LocalWorkspace;

#[derive(Default)]
struct RecordingRunner {
  calls: AtomicUsize,
  observed: Mutex<Vec<String>>,
  mutate_project: Option<PathBuf>,
  mutate_on_call: Option<usize>,
}

impl VerifierRunner for RecordingRunner {
  fn run(&self, request: &VerifierRun<'_>) -> anyhow::Result<ExecutedVerifier> {
    let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
    let candidate_file = request.candidate_root.join("candidate.txt");
    let content = fs::read_to_string(&candidate_file)?;
    self.observed.lock().unwrap().push(content);
    fs::write(candidate_file, "contaminated materialization")?;
    if self.mutate_on_call == Some(call)
      && let Some(root) = &self.mutate_project
    {
      fs::write(
        root.join("candidate.txt"),
        "candidate changed during verification",
      )?;
    }
    let context = tenet_domain::algebra::ExecutionContext {
      assurance: AssuranceProfileId(LOCAL_V1.into()),
      runner_semantics: RunnerSemanticsId(RUNNER_SEMANTICS_V1.into()),
      platform: PlatformInformation {
        os: "test".into(),
        architecture: "test".into(),
      },
      resolved_program: Some("test-runner".into()),
      resolved_program_digest: None,
    };
    Ok(ExecutedVerifier {
      observation: VerifierObservation {
        exit_code: Some(0),
        stdout: String::new(),
        stderr: String::new(),
        timed_out: false,
      },
      result: EvidenceResult::Pass,
      infrastructure_error: None,
      execution: ExecutionProvenance {
        runner_identity: RunnerIdentity("test-runner".into()),
        runner_semantics: context.runner_semantics.clone(),
        assurance: context.assurance.clone(),
        tenet_version: "test".into(),
        platform: context.platform.clone(),
        resolved_program: context.resolved_program.clone(),
        resolved_program_digest: None,
        execution_environment_identity: ExecutionEnvironmentIdentity(format!("call-{call}")),
      },
      context,
    })
  }
}
#[derive(Default)]
struct ErrorRunner {
  calls: AtomicUsize,
}

impl VerifierRunner for ErrorRunner {
  fn run(&self, _: &VerifierRun<'_>) -> anyhow::Result<ExecutedVerifier> {
    self.calls.fetch_add(1, Ordering::SeqCst);
    anyhow::bail!("runner unavailable")
  }
}

#[derive(Default)]
struct InconsistentRunner;

impl VerifierRunner for InconsistentRunner {
  fn run(&self, _: &VerifierRun<'_>) -> anyhow::Result<ExecutedVerifier> {
    let context = tenet_domain::algebra::ExecutionContext {
      assurance: AssuranceProfileId(LOCAL_V1.into()),
      runner_semantics: RunnerSemanticsId(RUNNER_SEMANTICS_V1.into()),
      platform: PlatformInformation {
        os: "test".into(),
        architecture: "test".into(),
      },
      resolved_program: None,
      resolved_program_digest: None,
    };
    Ok(ExecutedVerifier {
      observation: VerifierObservation {
        exit_code: Some(1),
        stdout: String::new(),
        stderr: String::new(),
        timed_out: false,
      },
      result: EvidenceResult::Pass,
      infrastructure_error: None,
      execution: ExecutionProvenance {
        runner_identity: RunnerIdentity("inconsistent".into()),
        runner_semantics: context.runner_semantics.clone(),
        assurance: context.assurance.clone(),
        tenet_version: "test".into(),
        platform: context.platform.clone(),
        resolved_program: None,
        resolved_program_digest: None,
        execution_environment_identity: ExecutionEnvironmentIdentity("inconsistent".into()),
      },
      context,
    })
  }
}

struct Fixture {
  directory: tempfile::TempDir,
  tenet: Tenet,
  runner: Arc<RecordingRunner>,
}

impl Fixture {
  fn new(mutate_on_call: Option<usize>) -> Self {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().to_path_buf();
    let runner = Arc::new(RecordingRunner {
      mutate_project: mutate_on_call.map(|_| root.clone()),
      mutate_on_call,
      ..RecordingRunner::default()
    });
    let tenet = Tenet::new(root.clone(), Arc::new(LocalWorkspace), runner.clone());
    tenet
      .initialize(&InitializeRequest { spec_path: None })
      .unwrap();
    fs::write(root.join("candidate.txt"), "original candidate").unwrap();
    let policy = ProjectConfig {
      version: 1,
      spec_path: "SPEC.md".into(),
      candidate: CandidateCapturePolicy {
        root: ".".into(),
        include: vec!["candidate.txt".into()],
        exclude: vec![],
      },
      verifiers: vec![verifier_spec("V1"), verifier_spec("V2")],
    };
    fs::write(
      root.join(".tenet/tenet.toml"),
      toml::to_string_pretty(&policy).unwrap(),
    )
    .unwrap();
    Self {
      directory,
      tenet,
      runner,
    }
  }

  fn root(&self) -> &Path {
    self.directory.path()
  }

  fn admit(&self) -> (AdmissionId, AuthorityId) {
    let proposal = self
      .tenet
      .authority_submit(AuthoritySubmitRequest::Proposal {
        contract: contract(),
        issues: vec![],
      })
      .unwrap();
    let AuthoritySubmissionResult::Proposal {
      proposal_id,
      authority_id,
      ..
    } = proposal
    else {
      panic!("expected proposal")
    };
    let reconciliation = self
      .tenet
      .authority_submit(AuthoritySubmitRequest::Reconciliation {
        proposal_id: proposal_id.clone(),
        findings: vec![],
      })
      .unwrap();
    let AuthoritySubmissionResult::Reconciliation {
      reconciliation_id, ..
    } = reconciliation
    else {
      panic!("expected reconciliation")
    };
    let admission = self
      .tenet
      .authority_submit(AuthoritySubmitRequest::Admission {
        proposal_id,
        reconciliation_id,
        authority_id: authority_id.clone(),
      })
      .unwrap();
    let AuthoritySubmissionResult::Admission { admission_id, .. } = admission else {
      panic!("expected admission")
    };
    (admission_id, authority_id)
  }
}

fn verifier_spec(id: &str) -> VerifierSpec {
  VerifierSpec {
    id: id.into(),
    command: CommandSpec {
      argv: vec![CommandArgument::Literal("true".into())],
      cwd: CommandCwd::Candidate(".".into()),
      env: EnvironmentSpec::default(),
      timeout_ms: 1_000,
      result: Default::default(),
    },
    max_output_bytes: 1_024,
    authority: VerifierAuthority::Project,
    oracle_path: None,
  }
}

fn contract() -> CompletionContractV1 {
  CompletionContractV1 {
    schema_version: 1,
    policy: CompletionPolicyId(COMPLETION_POLICY_V1.into()),
    requirements: vec![Requirement {
      id: RequirementId("R1".into()),
      statement: "candidate behavior is complete".into(),
      criteria: vec![criterion("C1", "V1"), criterion("C2", "V2")],
    }],
  }
}

fn criterion(id: &str, verifier: &str) -> Criterion {
  Criterion {
    id: CriterionId(id.into()),
    proposition: format!("criterion {id} holds"),
    verifiers: vec![Verifier {
      id: VerifierId(verifier.into()),
      material: VerifierMaterial::Candidate,
    }],
    evidence: EvidenceRequirementV1 {
      control: EvidenceControlRequirementV1::CandidateControlledPermitted,
      assurance: AssuranceRequirementV1::LocalOrStronger,
    },
  }
}

#[test]
fn context_derives_authority_lifecycle_and_completed_from_refs() {
  let fixture = Fixture::new(None);
  assert_eq!(
    fixture.tenet.context().unwrap().phase,
    WorkflowPhase::AuthorityRequired
  );

  let proposal = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Proposal {
      contract: contract(),
      issues: vec![],
    })
    .unwrap();
  let AuthoritySubmissionResult::Proposal {
    proposal_id,
    authority_id,
    ..
  } = proposal
  else {
    panic!("expected proposal")
  };
  assert_eq!(
    fixture.tenet.context().unwrap().phase,
    WorkflowPhase::AuthorityReconciliation
  );

  let reconciliation = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Reconciliation {
      proposal_id: proposal_id.clone(),
      findings: vec![],
    })
    .unwrap();
  let AuthoritySubmissionResult::Reconciliation {
    reconciliation_id, ..
  } = reconciliation
  else {
    panic!("expected reconciliation")
  };
  assert_eq!(
    fixture.tenet.context().unwrap().phase,
    WorkflowPhase::AuthorityAdmission
  );

  fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Admission {
      proposal_id,
      reconciliation_id,
      authority_id,
    })
    .unwrap();
  assert_eq!(
    fixture.tenet.context().unwrap().phase,
    WorkflowPhase::Implementation
  );
  assert_eq!(fixture.tenet.verify().unwrap().verdict, Verdict::Done);
  assert_eq!(
    fixture.tenet.context().unwrap().phase,
    WorkflowPhase::Completed
  );
  assert!(!fixture.root().join(".tenet/phase").exists());
}

#[test]
fn requirement_check_is_development_only_and_verify_reruns_every_verifier() {
  let fixture = Fixture::new(None);
  fixture.admit();
  let checked = fixture
    .tenet
    .requirement_check(&RequirementCheckRequest {
      requirement_id: RequirementId("R1".into()),
    })
    .unwrap();
  assert_eq!(checked.result.state, CompletionState::Satisfied);
  assert_eq!(checked.result.verdict, None);
  assert!(!fixture.root().join(".tenet/refs/final").exists());
  assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 2);
  let context = fixture.tenet.context().unwrap();
  assert_eq!(context.phase, WorkflowPhase::Implementation);
  assert_eq!(context.requirement_checks.len(), 1);
  assert_eq!(
    context.requirement_checks[0].evaluation_id,
    checked.evaluation_id
  );
  assert_eq!(
    context.requirement_checks[0].candidate_id,
    checked.candidate_id
  );

  let verified = fixture.tenet.verify().unwrap();
  assert_eq!(verified.verdict, Verdict::Done);
  assert_eq!(fixture.runner.calls.load(Ordering::SeqCst), 4);
  let observed = fixture.runner.observed.lock().unwrap();
  assert_eq!(observed.as_slice(), ["original candidate"; 4]);

  let final_id = LocalWorkspace
    .read_ref(fixture.root(), "final")
    .unwrap()
    .unwrap();
  let bytes = LocalWorkspace
    .load_object(fixture.root(), &final_id)
    .unwrap();
  let evaluation: Evaluation = serde_json::from_slice(&bytes).unwrap();
  assert_eq!(final_id, verified.evaluation_id.0);
  assert_eq!(evaluation.admission, verified.admission_id);
  assert_eq!(evaluation.authority, verified.authority_id);
  assert_eq!(evaluation.candidate, verified.candidate_id);
  assert!(matches!(evaluation.scope, EvaluationScope::Final));
  assert_eq!(evaluation.runs.len(), 2);
  assert!(evaluation.runs.iter().all(|run| {
    run.authority == verified.authority_id && run.candidate == verified.candidate_id
  }));
}

#[test]
fn successful_historical_evaluation_does_not_complete_mutated_candidate() {
  let fixture = Fixture::new(Some(1));
  fixture.admit();
  let result = fixture.tenet.verify().unwrap();
  assert_eq!(result.verdict, Verdict::Inconclusive);
  assert_eq!(
    result.reason.as_deref(),
    Some("CANDIDATE_CHANGED_DURING_VERIFICATION")
  );
  assert_ne!(
    result.current_candidate_id.as_ref(),
    Some(&result.candidate_id)
  );
  assert_eq!(result.result.verdict, Some(Verdict::Done));
  assert_eq!(
    fixture.tenet.context().unwrap().phase,
    WorkflowPhase::Implementation
  );
}

#[test]
fn exact_authority_stage_bindings_cannot_transfer() {
  let fixture = Fixture::new(None);
  let first = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Proposal {
      contract: contract(),
      issues: vec![],
    })
    .unwrap();
  let AuthoritySubmissionResult::Proposal {
    proposal_id: p1,
    authority_id: a1,
    ..
  } = first
  else {
    panic!("expected proposal")
  };
  let report = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Reconciliation {
      proposal_id: p1.clone(),
      findings: vec![],
    })
    .unwrap();
  let AuthoritySubmissionResult::Reconciliation {
    reconciliation_id: r1,
    ..
  } = report
  else {
    panic!("expected reconciliation")
  };
  fs::write(fixture.root().join("SPEC.md"), "changed specification").unwrap();
  let second = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Proposal {
      contract: contract(),
      issues: vec![],
    })
    .unwrap();
  let AuthoritySubmissionResult::Proposal {
    proposal_id: p2,
    authority_id: a2,
    ..
  } = second
  else {
    panic!("expected proposal")
  };
  assert_ne!(p1, p2);
  assert_ne!(a1, a2);
  let error = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Admission {
      proposal_id: p2,
      reconciliation_id: r1,
      authority_id: a2,
    })
    .unwrap_err();
  assert_eq!(error.code, "admission_invalid");
}

#[test]
fn unknown_completion_and_candidate_semantics_fail_closed() {
  let fixture = Fixture::new(None);
  let mut unknown = contract();
  unknown.policy = CompletionPolicyId("tenet:completion-policy:v999".into());
  let error = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Proposal {
      contract: unknown,
      issues: vec![],
    })
    .unwrap_err();
  assert_eq!(error.code, "semantics_incompatible");

  let manifest = TreeManifest {
    version: 1,
    semantics: SnapshotSemanticsId("tenet:candidate-semantics:v999".into()),
    entries: vec![],
  };
  let id = LocalWorkspace
    .store_object(fixture.root(), &serde_json::to_vec(&manifest).unwrap())
    .unwrap();
  assert!(LocalWorkspace.materialize(fixture.root(), &id).is_err());
}

#[test]
fn authority_submit_rejects_producer_supplied_verdict() {
  let value = serde_json::json!({
    "stage": "PROPOSAL",
    "contract": contract(),
    "issues": [],
    "verdict": "DONE"
  });
  assert!(serde_json::from_value::<AuthoritySubmitRequest>(value).is_err());
}

#[test]
fn doctor_validates_initialized_repository_invariants() {
  let fixture = Fixture::new(None);
  fixture.admit();
  let result = fixture.tenet.doctor().unwrap();
  assert!(result.healthy, "{result:#?}");
  let names = result
    .checks
    .iter()
    .map(|check| check.name.as_str())
    .collect::<Vec<_>>();
  assert_eq!(
    names,
    [
      "repository_root",
      "specification",
      "object_blob_ref_integrity",
      "active_admission_chain",
      "supported_semantic_versions",
      "repository_write_scope",
      "integration_consistency",
    ]
  );
  for path in [
    ".tenet/format",
    ".tenet/.gitignore",
    ".tenet/objects",
    ".tenet/blobs",
    ".tenet/refs",
    ".tenet/refs/proposal",
    ".tenet/refs/reconciliation",
    ".tenet/refs/active-admission",
    ".tenet/refs/requirements",
    ".tenet/tmp",
    ".tenet/lock",
  ] {
    assert!(fixture.root().join(path).exists(), "missing {path}");
  }
}

#[test]
fn blocking_reconciliation_derives_clarification_and_stale_admission_is_detected() {
  let fixture = Fixture::new(None);
  let proposal = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Proposal {
      contract: contract(),
      issues: vec![],
    })
    .unwrap();
  let AuthoritySubmissionResult::Proposal { proposal_id, .. } = proposal else {
    panic!("expected proposal")
  };
  fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Reconciliation {
      proposal_id,
      findings: vec![tenet_domain::authority::Finding {
        code: "ambiguity".into(),
        message: "clarification required".into(),
        blocking: true,
      }],
    })
    .unwrap();
  assert_eq!(
    fixture.tenet.context().unwrap().phase,
    WorkflowPhase::AuthorityClarification
  );

  let admitted = Fixture::new(None);
  admitted.admit();
  fs::write(admitted.root().join("SPEC.md"), "stale").unwrap();
  assert_eq!(
    admitted.tenet.context().unwrap().phase,
    WorkflowPhase::AuthorityStale
  );
}

#[test]
fn candidate_controlled_verifier_requires_explicit_contract_policy() {
  let fixture = Fixture::new(None);
  let mut contract = contract();
  contract.requirements[0].criteria[0].verifiers[0].material = VerifierMaterial::AuthorityBundle;
  let error = fixture
    .tenet
    .authority_submit(AuthoritySubmitRequest::Proposal {
      contract,
      issues: vec![],
    })
    .unwrap_err();
  assert_eq!(error.code, "verifier_material_mismatch");
}

#[test]
fn unknown_repository_format_is_not_reinterpreted() {
  let fixture = Fixture::new(None);
  fs::write(fixture.root().join(".tenet/format"), "999\n").unwrap();
  let result = fixture.tenet.doctor().unwrap();
  assert!(!result.healthy);
  assert!(
    result
      .checks
      .iter()
      .any(|check| check.name == "object_blob_ref_integrity" && !check.passed)
  );
}

#[test]
fn admitted_context_ignores_mutable_live_policy_redirection() {
  let fixture = Fixture::new(None);
  let (admission_id, authority_id) = fixture.admit();
  fs::write(
    fixture.root().join(".tenet/tenet.toml"),
    "version = 999\nspec_path = \"missing.md\"\n",
  )
  .unwrap();
  let context = fixture.tenet.context().unwrap();
  assert_eq!(context.phase, WorkflowPhase::Implementation);
  assert_eq!(context.active_admission_id, Some(admission_id));
  assert_eq!(context.authority_id, Some(authority_id));
}

#[test]
fn runner_errors_are_persisted_for_every_required_verifier() {
  let fixture = Fixture::new(None);
  fixture.admit();
  let runner = Arc::new(ErrorRunner::default());
  let tenet = Tenet::new(
    fixture.root().to_path_buf(),
    Arc::new(LocalWorkspace),
    runner.clone(),
  );
  let result = tenet.verify().unwrap();
  assert_eq!(result.verdict, Verdict::InfrastructureError);
  assert_eq!(runner.calls.load(Ordering::SeqCst), 2);
  let bytes = LocalWorkspace
    .load_object(fixture.root(), &result.evaluation_id.0)
    .unwrap();
  let evaluation: Evaluation = serde_json::from_slice(&bytes).unwrap();
  assert_eq!(evaluation.runs.len(), 2);
  assert!(evaluation.runs.iter().all(|run| {
    run.observation.result == EvidenceResult::InfrastructureError
      && run.authority == result.authority_id
      && run.candidate == result.candidate_id
  }));
}

#[test]
fn inconsistent_runner_claim_cannot_produce_done() {
  let fixture = Fixture::new(None);
  fixture.admit();
  let tenet = Tenet::new(
    fixture.root().to_path_buf(),
    Arc::new(LocalWorkspace),
    Arc::new(InconsistentRunner),
  );
  assert_eq!(
    tenet.verify().unwrap().verdict,
    Verdict::InfrastructureError
  );
}

#[test]
fn unknown_draft_lifecycle_version_derives_incompatible() {
  let fixture = Fixture::new(None);
  let root = fixture.root().canonicalize().unwrap();
  let proposal = tenet_domain::authority::AuthorityProposal {
    schema_version: 999,
    authority: AuthorityId(tenet_domain::evidence::ContentObjectId(format!(
      "sha256:{}",
      "a".repeat(64)
    ))),
    issues: vec![],
  };
  let id = LocalWorkspace
    .store_object(&root, &serde_json::to_vec(&proposal).unwrap())
    .unwrap();
  LocalWorkspace.write_ref(&root, "proposal", &id).unwrap();
  assert_eq!(
    fixture.tenet.context().unwrap().phase,
    WorkflowPhase::Incompatible
  );
}
