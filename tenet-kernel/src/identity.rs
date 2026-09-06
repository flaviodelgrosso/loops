//! Deterministic content identity derivation and Candidate manifest validation.

use std::path::{Component, Path};
use tenet_domain::{
  evidence::ContentObjectId,
  snapshot::{CANDIDATE_SEMANTICS_V1, EntryKind, TreeEntry, TreeManifest},
};
use thiserror::Error;

use crate::digest::canonical_digest;

#[derive(Debug, Error)]
pub enum IdentityError {
  #[error("sealed authority oracle bundle is not a directory")]
  BundleNotDirectory,
  #[error("canonicalize oracle bundle: {0}")]
  Canonical(#[from] serde_json::Error),
  #[error("{0}")]
  Digest(String),
  #[error("snapshot does not declare CandidateSemantics v1")]
  InvalidCandidateSemantics,
  #[error("candidate manifest path is not normalized: {0}")]
  NonNormalizedCandidatePath(String),
  #[error("candidate manifest contains reserved path: {0}")]
  ReservedCandidatePath(String),
  #[error("candidate manifest contains an empty directory: {0}")]
  EmptyCandidateDirectory(String),
  #[error("candidate manifest entry is malformed: {0}")]
  MalformedCandidateEntry(String),
  #[error("candidate manifest contains a canonical path collision: {0}")]
  CandidatePathCollision(String),
}

pub fn validate_candidate_manifest(manifest: &TreeManifest) -> Result<(), IdentityError> {
  if manifest.semantics.0 != CANDIDATE_SEMANTICS_V1 {
    return Err(IdentityError::InvalidCandidateSemantics);
  }
  let mut previous: Option<&str> = None;
  for entry in &manifest.entries {
    if !normalized_candidate_path(&entry.path) {
      return Err(IdentityError::NonNormalizedCandidatePath(
        entry.path.clone(),
      ));
    }
    if crate::policy::candidate_path_is_reserved(&entry.path) {
      return Err(IdentityError::ReservedCandidatePath(entry.path.clone()));
    }
    if previous.is_some_and(|path| path >= entry.path.as_str()) {
      return Err(IdentityError::CandidatePathCollision(entry.path.clone()));
    }
    previous = Some(&entry.path);
    match entry.kind {
      EntryKind::File if entry.content_id.is_some() => {}
      EntryKind::Directory => {
        return Err(IdentityError::EmptyCandidateDirectory(entry.path.clone()));
      }
      EntryKind::File => return Err(IdentityError::MalformedCandidateEntry(entry.path.clone())),
    }
  }
  Ok(())
}

fn normalized_candidate_path(value: &str) -> bool {
  let path = Path::new(value);
  !value.is_empty()
    && !value.contains('\\')
    && !value.contains("//")
    && !value.starts_with("./")
    && !value.contains("/./")
    && !value.ends_with("/.")
    && !value.ends_with('/')
    && !path.is_absolute()
    && path
      .components()
      .all(|component| matches!(component, Component::Normal(_)))
}

pub fn subtree_content_id(
  entries: &[TreeEntry],
  path: &str,
) -> Result<ContentObjectId, IdentityError> {
  let prefix = format!("{path}/");
  let entries = entries
    .iter()
    .filter(|entry| entry.path == path || entry.path.starts_with(&prefix))
    .collect::<Vec<_>>();
  if entries
    .first()
    .is_none_or(|entry| entry.kind != EntryKind::Directory)
  {
    return Err(IdentityError::BundleNotDirectory);
  }
  ContentObjectId::new(canonical_digest(&entries)?).map_err(IdentityError::Digest)
}

pub fn sealed_executable_content_id(
  entries: &[TreeEntry],
  executable_path: &str,
) -> Option<ContentObjectId> {
  entries
    .iter()
    .find(|entry| entry.path == executable_path && entry.kind == EntryKind::File)
    .and_then(|entry| entry.content_id.clone())
}

#[cfg(test)]
mod tests {
  use tenet_domain::{
    evidence::ContentObjectId,
    snapshot::{CANDIDATE_SEMANTICS_V1, EntryKind, SnapshotSemanticsId, TreeEntry, TreeManifest},
  };

  use super::{IdentityError, validate_candidate_manifest};

  fn file(path: &str) -> TreeEntry {
    TreeEntry {
      path: path.into(),
      kind: EntryKind::File,
      content_id: Some(ContentObjectId(
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
      )),
      executable: false,
    }
  }

  fn manifest(entries: Vec<TreeEntry>) -> TreeManifest {
    TreeManifest {
      version: 1,
      semantics: SnapshotSemanticsId(CANDIDATE_SEMANTICS_V1.into()),
      entries,
    }
  }

  #[test]
  fn reserved_paths_are_rejected_even_with_candidate_semantics() {
    assert!(matches!(
      validate_candidate_manifest(&manifest(vec![file(".git/config")])),
      Err(IdentityError::ReservedCandidatePath(_))
    ));
  }

  #[test]
  fn empty_directories_are_rejected_even_with_candidate_semantics() {
    assert!(matches!(
      validate_candidate_manifest(&manifest(vec![TreeEntry {
        path: "empty".into(),
        kind: EntryKind::Directory,
        content_id: None,
        executable: false,
      }])),
      Err(IdentityError::EmptyCandidateDirectory(_))
    ));
  }

  #[test]
  fn canonical_path_collisions_are_rejected() {
    assert!(matches!(
      validate_candidate_manifest(&manifest(vec![file("same"), file("same")])),
      Err(IdentityError::CandidatePathCollision(_))
    ));
  }
}
