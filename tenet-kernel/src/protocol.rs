//! Pure workflow phase derivation from persisted facts.

use tenet_domain::protocol::{ContextFacts, WorkflowPhase};

pub const fn derive_phase(facts: ContextFacts) -> WorkflowPhase {
  if !facts.spec_exists {
    WorkflowPhase::SpecRequired
  } else if !facts.compatible {
    WorkflowPhase::Incompatible
  } else if !facts.proposal_exists {
    WorkflowPhase::AuthorityRequired
  } else if !facts.reconciliation_exists {
    WorkflowPhase::AuthorityReconciliation
  } else if facts.reconciliation_blocked {
    WorkflowPhase::AuthorityClarification
  } else if !facts.admission_exists {
    WorkflowPhase::AuthorityAdmission
  } else if !facts.authority_current {
    WorkflowPhase::AuthorityStale
  } else if facts.final_satisfied && facts.current_matches_final {
    WorkflowPhase::Completed
  } else {
    WorkflowPhase::Implementation
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn implementation() -> ContextFacts {
    ContextFacts {
      spec_exists: true,
      compatible: true,
      proposal_exists: true,
      reconciliation_exists: true,
      reconciliation_blocked: false,
      admission_exists: true,
      authority_current: true,
      final_satisfied: false,
      current_matches_final: false,
    }
  }

  #[test]
  fn persisted_fact_precedence_derives_each_blocking_phase() {
    let mut facts = implementation();
    facts.spec_exists = false;
    assert_eq!(derive_phase(facts), WorkflowPhase::SpecRequired);

    let mut facts = implementation();
    facts.compatible = false;
    assert_eq!(derive_phase(facts), WorkflowPhase::Incompatible);

    let mut facts = implementation();
    facts.proposal_exists = false;
    assert_eq!(derive_phase(facts), WorkflowPhase::AuthorityRequired);

    let mut facts = implementation();
    facts.reconciliation_exists = false;
    assert_eq!(derive_phase(facts), WorkflowPhase::AuthorityReconciliation);

    let mut facts = implementation();
    facts.reconciliation_blocked = true;
    assert_eq!(derive_phase(facts), WorkflowPhase::AuthorityClarification);

    let mut facts = implementation();
    facts.admission_exists = false;
    assert_eq!(derive_phase(facts), WorkflowPhase::AuthorityAdmission);

    let mut facts = implementation();
    facts.authority_current = false;
    assert_eq!(derive_phase(facts), WorkflowPhase::AuthorityStale);
  }

  #[test]
  fn completed_requires_successful_final_for_current_candidate() {
    let mut facts = implementation();
    facts.final_satisfied = true;
    assert_eq!(derive_phase(facts), WorkflowPhase::Implementation);

    facts.current_matches_final = true;
    assert_eq!(derive_phase(facts), WorkflowPhase::Completed);
  }
}
