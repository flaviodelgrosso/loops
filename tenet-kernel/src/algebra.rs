//! CompletionPolicy v1 admission and evaluation semantics.

use std::collections::{BTreeMap, BTreeSet};

use tenet_domain::{
  algebra::{
    AlgebraError, AssuranceProfileId, AssuranceRequirementV1, COMPLETION_POLICY_V1,
    CompletionContractV1, CompletionEvaluation, CompletionPolicyId, CompletionState,
    CriterionEvaluation, CriterionState, Evaluation, EvaluationScope, EvidenceControl,
    EvidenceControlRequirementV1, EvidenceResult, LOCAL_V1, PROTECTED_V1, RUNNER_SEMANTICS_V1,
    RequirementEvaluation, VerifierRun,
  },
  authority::{Admission, Authority, AuthorityProposal, ReconciliationReport, SpecSnapshot},
  completion::Verdict,
};

use crate::{
  authority::{admission_id, authority_id, validate_admission},
  digest::canonical_digest,
};

pub struct AdmissionChain<'a> {
  pub admission: &'a Admission,
  pub proposal: &'a AuthorityProposal,
  pub report: &'a ReconciliationReport,
  pub authority: &'a Authority,
  pub spec: &'a SpecSnapshot,
}

pub fn validate_contract(contract: &CompletionContractV1) -> Result<(), AlgebraError> {
  if contract.schema_version != 1 {
    return Err(AlgebraError::UnsupportedSchemaVersion(
      contract.schema_version,
    ));
  }
  require_policy_v1(&contract.policy)?;
  if contract.requirements.is_empty() {
    return Err(AlgebraError::MissingRequirements);
  }

  let mut requirement_ids = BTreeSet::new();
  let mut criterion_ids = BTreeSet::new();
  let mut verifier_ids = BTreeSet::new();
  for requirement in &contract.requirements {
    validate_id("requirement", &requirement.id.0)?;
    validate_statement("requirement", &requirement.id.0, &requirement.statement)?;
    if !requirement_ids.insert(requirement.id.0.as_str()) {
      return Err(AlgebraError::DuplicateId {
        kind: "requirement",
        id: requirement.id.0.clone(),
      });
    }
    if requirement.criteria.is_empty() {
      return Err(AlgebraError::MissingCriteria(requirement.id.0.clone()));
    }
    for criterion in &requirement.criteria {
      validate_id("criterion", &criterion.id.0)?;
      validate_statement("criterion", &criterion.id.0, &criterion.proposition)?;
      if !criterion_ids.insert(criterion.id.0.as_str()) {
        return Err(AlgebraError::DuplicateId {
          kind: "criterion",
          id: criterion.id.0.clone(),
        });
      }
      if criterion.verifiers.is_empty() {
        return Err(AlgebraError::MissingVerifiers(criterion.id.0.clone()));
      }
      for verifier in &criterion.verifiers {
        validate_id("verifier", &verifier.id.0)?;
        if !verifier_ids.insert(verifier.id.0.as_str()) {
          return Err(AlgebraError::DuplicateId {
            kind: "verifier",
            id: verifier.id.0.clone(),
          });
        }
      }
      let authority_bound = criterion
        .verifiers
        .iter()
        .filter(|verifier| verifier.material.control() == EvidenceControl::AuthorityBound)
        .count();
      let possible = match criterion.evidence.control {
        EvidenceControlRequirementV1::AuthorityBoundOnly => {
          authority_bound == criterion.verifiers.len()
        }
        EvidenceControlRequirementV1::AtLeastOneAuthorityBound => authority_bound > 0,
        EvidenceControlRequirementV1::CandidateControlledPermitted => true,
      };
      if !possible {
        return Err(AlgebraError::ImpossibleEvidenceControl(
          criterion.id.0.clone(),
        ));
      }
    }
  }
  Ok(())
}

pub fn validate_completion_admission(
  contract: &CompletionContractV1,
  chain: &AdmissionChain<'_>,
) -> Result<(), AlgebraError> {
  validate_contract(contract)?;
  if canonical_digest(contract).ok().as_deref() != Some(&chain.authority.contract.0) {
    return Err(AlgebraError::ContractMismatch);
  }
  validate_admission(
    chain.admission,
    chain.proposal,
    chain.report,
    chain.authority,
    chain.spec,
  )?;
  Ok(())
}

pub fn evidence_result(
  observation: &tenet_domain::algebra::ExecutionObservation,
) -> EvidenceResult {
  if observation.infrastructure_error.is_some()
    || observation.timed_out
    || observation.exit_code.is_none()
  {
    EvidenceResult::InfrastructureError
  } else {
    observation.result
  }
}

pub fn evaluate(
  contract: &CompletionContractV1,
  chain: &AdmissionChain<'_>,
  evaluation: &Evaluation,
) -> Result<CompletionEvaluation, AlgebraError> {
  validate_completion_admission(contract, chain)?;
  if admission_id(chain.admission).ok().as_ref() != Some(&evaluation.admission) {
    return Err(AlgebraError::AdmissionMismatch);
  }
  if authority_id(chain.authority).ok().as_ref() != Some(&evaluation.authority) {
    return Err(AlgebraError::AuthorityMismatch);
  }

  let requirements = scoped_requirements(contract, &evaluation.scope)?;
  let mut expected = BTreeMap::new();
  for requirement in &requirements {
    for criterion in &requirement.criteria {
      for verifier in &criterion.verifiers {
        expected.insert(verifier.id.clone(), criterion);
      }
    }
  }

  let runs = validate_runs(evaluation, &expected)?;
  let mut requirement_results = Vec::with_capacity(requirements.len());
  for requirement in requirements {
    let mut criteria = Vec::with_capacity(requirement.criteria.len());
    for criterion in &requirement.criteria {
      let mut saw_failure = false;
      let mut saw_infrastructure = false;
      let mut saw_missing = false;
      let mut saw_inadmissible = false;
      let mut saw_inconclusive = false;
      for verifier in &criterion.verifiers {
        let Some(run) = runs.get(&verifier.id) else {
          saw_missing = true;
          continue;
        };
        let assurance_admissible =
          assurance_satisfies(&run.context.assurance, criterion.evidence.assurance)?;
        match evidence_result(&run.observation) {
          EvidenceResult::Fail => saw_failure = true,
          EvidenceResult::InfrastructureError => saw_infrastructure = true,
          EvidenceResult::Pass if !assurance_admissible => saw_inadmissible = true,
          EvidenceResult::Inconclusive if !assurance_admissible => saw_inadmissible = true,
          EvidenceResult::Pass => {}
          EvidenceResult::Inconclusive => saw_inconclusive = true,
        }
      }
      let state = if saw_failure {
        CriterionState::Contradicted
      } else if saw_infrastructure {
        CriterionState::InfrastructureError
      } else if saw_missing {
        CriterionState::MissingEvidence
      } else if saw_inadmissible {
        CriterionState::InadmissibleEvidence
      } else if saw_inconclusive {
        CriterionState::Inconclusive
      } else {
        CriterionState::Satisfied
      };
      criteria.push(CriterionEvaluation {
        criterion: criterion.id.clone(),
        state,
      });
    }
    requirement_results.push(RequirementEvaluation {
      requirement: requirement.id.clone(),
      state: aggregate(criteria.iter().map(|criterion| criterion.state)),
      criteria,
    });
  }

  let state = aggregate(
    requirement_results
      .iter()
      .map(|requirement| match requirement.state {
        CompletionState::Satisfied => CriterionState::Satisfied,
        CompletionState::Contradicted => CriterionState::Contradicted,
        CompletionState::Inconclusive => CriterionState::Inconclusive,
        CompletionState::InfrastructureError => CriterionState::InfrastructureError,
      }),
  );
  let verdict = matches!(evaluation.scope, EvaluationScope::Final).then(|| match state {
    CompletionState::Satisfied => Verdict::Done,
    CompletionState::Contradicted => Verdict::NotDone,
    CompletionState::Inconclusive => Verdict::Inconclusive,
    CompletionState::InfrastructureError => Verdict::InfrastructureError,
  });
  Ok(CompletionEvaluation {
    state,
    requirements: requirement_results,
    verdict,
  })
}

fn require_policy_v1(policy: &CompletionPolicyId) -> Result<(), AlgebraError> {
  if policy.0 == COMPLETION_POLICY_V1 {
    Ok(())
  } else {
    Err(AlgebraError::UnsupportedCompletionPolicy(policy.0.clone()))
  }
}

fn scoped_requirements<'a>(
  contract: &'a CompletionContractV1,
  scope: &EvaluationScope,
) -> Result<Vec<&'a tenet_domain::algebra::Requirement>, AlgebraError> {
  match scope {
    EvaluationScope::Final => Ok(contract.requirements.iter().collect()),
    EvaluationScope::Requirement { requirement } => contract
      .requirements
      .iter()
      .find(|item| item.id == *requirement)
      .map(|item| vec![item])
      .ok_or_else(|| AlgebraError::UnknownRequirement(requirement.0.clone())),
  }
}

fn validate_runs<'a>(
  evaluation: &'a Evaluation,
  expected: &BTreeMap<tenet_domain::algebra::VerifierId, &tenet_domain::algebra::Criterion>,
) -> Result<BTreeMap<tenet_domain::algebra::VerifierId, &'a VerifierRun>, AlgebraError> {
  let mut runs = BTreeMap::new();
  for run in &evaluation.runs {
    if run.authority != evaluation.authority {
      return Err(AlgebraError::RunAuthorityMismatch);
    }
    if run.candidate != evaluation.candidate {
      return Err(AlgebraError::RunCandidateMismatch);
    }
    if run.context.runner_semantics.0 != RUNNER_SEMANTICS_V1 {
      return Err(AlgebraError::UnsupportedRunnerSemantics(
        run.context.runner_semantics.0.clone(),
      ));
    }
    if !expected.contains_key(&run.verifier) {
      return Err(AlgebraError::VerifierOutsideScope(run.verifier.0.clone()));
    }
    if runs.insert(run.verifier.clone(), run).is_some() {
      return Err(AlgebraError::DuplicateRun(run.verifier.0.clone()));
    }
  }
  Ok(runs)
}

fn assurance_satisfies(
  profile: &AssuranceProfileId,
  requirement: AssuranceRequirementV1,
) -> Result<bool, AlgebraError> {
  match profile.0.as_str() {
    LOCAL_V1 => Ok(requirement == AssuranceRequirementV1::LocalOrStronger),
    PROTECTED_V1 => Ok(true),
    _ => Err(AlgebraError::UnsupportedAssuranceProfile(profile.0.clone())),
  }
}

fn validate_id(kind: &'static str, id: &str) -> Result<(), AlgebraError> {
  if id.trim().is_empty() {
    Err(AlgebraError::BlankId { kind })
  } else {
    Ok(())
  }
}

fn validate_statement(kind: &'static str, id: &str, value: &str) -> Result<(), AlgebraError> {
  if value.trim().is_empty() {
    Err(AlgebraError::BlankStatement {
      kind,
      id: id.to_owned(),
    })
  } else {
    Ok(())
  }
}

fn aggregate(states: impl Iterator<Item = CriterionState>) -> CompletionState {
  let mut saw_infrastructure = false;
  let mut saw_inconclusive = false;
  for state in states {
    match state {
      CriterionState::Contradicted => return CompletionState::Contradicted,
      CriterionState::InfrastructureError => saw_infrastructure = true,
      CriterionState::Satisfied => {}
      CriterionState::MissingEvidence
      | CriterionState::InadmissibleEvidence
      | CriterionState::Inconclusive => saw_inconclusive = true,
    }
  }
  if saw_infrastructure {
    CompletionState::InfrastructureError
  } else if saw_inconclusive {
    CompletionState::Inconclusive
  } else {
    CompletionState::Satisfied
  }
}
