//! Vocabulary for captured content-addressed snapshot trees.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::evidence::ContentObjectId;

pub const CANDIDATE_SEMANTICS_V1: &str = "tenet:candidate-semantics:v1";
pub const AUTHORITY_SNAPSHOT_SEMANTICS_V1: &str = "tenet:authority-snapshot-semantics:v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SnapshotSemanticsId(pub String);

pub type CandidateSemanticsId = SnapshotSemanticsId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TreeManifest {
  pub version: u32,
  pub semantics: SnapshotSemanticsId,
  pub entries: Vec<TreeEntry>,
}

/// Manifest whose canonical digest identifies a Candidate under its declared semantics.
pub type CandidateManifest = TreeManifest;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TreeEntry {
  pub path: String,
  pub kind: EntryKind,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub content_id: Option<ContentObjectId>,
  pub executable: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
  Directory,
  File,
}
