use tenet_domain::policy::{CandidateCapturePolicy, PolicyError, ProjectConfig};
use tenet_kernel::policy::{candidate_excludes, validate_candidate_surface, validate_policy};

fn policy(candidate: CandidateCapturePolicy) -> ProjectConfig {
  ProjectConfig {
    version: 1,
    spec_path: "SPEC.md".into(),
    candidate,
    verifiers: Vec::new(),
  }
}

#[test]
fn candidate_exclusions_match_only_declared_relative_rules() {
  let candidate = CandidateCapturePolicy {
    root: ".".into(),
    include: vec!["build/**".into()],
    exclude: vec!["build/**".into(), "notes.txt".into()],
  };
  assert!(candidate_excludes(&candidate, ".tenet/refs/final"));
  assert!(candidate_excludes(&candidate, "build/cache/result"));
  assert!(candidate_excludes(&candidate, "notes.txt"));
  assert!(!candidate_excludes(&candidate, "build-output/result"));
  assert!(!candidate_excludes(&candidate, "notes.txt.bak"));
}

#[test]
fn candidate_boundary_rejects_unsafe_paths_and_rules() {
  assert_eq!(
    validate_policy(&policy(CandidateCapturePolicy {
      root: ".tenet".into(),
      ..CandidateCapturePolicy::default()
    })),
    Err(PolicyError::InvalidCandidateRoot)
  );
  assert_eq!(
    validate_policy(&policy(CandidateCapturePolicy {
      exclude: vec!["../outside/**".into()],
      ..CandidateCapturePolicy::default()
    })),
    Err(PolicyError::InvalidCandidateExclusion(
      "../outside/**".into()
    ))
  );
  assert_eq!(
    validate_policy(&policy(CandidateCapturePolicy {
      exclude: vec!["./build/**".into()],
      ..CandidateCapturePolicy::default()
    })),
    Err(PolicyError::InvalidCandidateExclusion("./build/**".into()))
  );
  assert_eq!(
    validate_policy(&policy(CandidateCapturePolicy {
      exclude: vec!["build//cache".into()],
      ..CandidateCapturePolicy::default()
    })),
    Err(PolicyError::InvalidCandidateExclusion(
      "build//cache".into()
    ))
  );
  assert_eq!(
    validate_policy(&policy(CandidateCapturePolicy {
      exclude: vec![".".into()],
      ..CandidateCapturePolicy::default()
    })),
    Err(PolicyError::InvalidCandidateExclusion(".".into()))
  );
}

#[test]
fn candidate_surface_allows_missing_future_paths_but_requires_explicit_include() {
  assert_eq!(
    validate_candidate_surface(&CandidateCapturePolicy::default()),
    Err(PolicyError::CandidateSurfaceUnconfigured)
  );
  assert!(
    validate_policy(&policy(CandidateCapturePolicy {
      include: vec!["Cargo.toml".into(), "src/**".into()],
      ..CandidateCapturePolicy::default()
    }))
    .is_ok()
  );
}

#[test]
fn candidate_selectors_reject_unsafe_and_tenet_paths() {
  for selector in [
    "../outside",
    "./src",
    "src/*",
    "src/**/generated",
    ".tenet/**",
  ] {
    assert_eq!(
      validate_policy(&policy(CandidateCapturePolicy {
        include: vec![selector.into()],
        ..CandidateCapturePolicy::default()
      })),
      Err(PolicyError::InvalidCandidateInclusion(selector.into()))
    );
  }
  assert_eq!(
    validate_policy(&policy(CandidateCapturePolicy {
      exclude: vec![".tenet/**".into()],
      ..CandidateCapturePolicy::default()
    })),
    Err(PolicyError::InvalidCandidateExclusion(".tenet/**".into()))
  );
  assert!(
    validate_policy(&policy(CandidateCapturePolicy {
      include: vec!["**".into()],
      exclude: vec!["build/**".into(), "notes.txt".into()],
      ..CandidateCapturePolicy::default()
    }))
    .is_ok()
  );
}
