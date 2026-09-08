use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
  name = "tenet",
  about = "Completion authority for exact admitted Authority and Candidate identities"
)]
pub struct Cli {
  #[arg(long, global = true, value_name = "DIR")]
  pub cwd: Option<PathBuf>,
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
  /// Initialize repository-contained Tenet state and integrations.
  Init {
    #[arg(long, value_name = "PATH")]
    spec: Option<PathBuf>,
    #[arg(long)]
    json: bool,
  },
  /// Validate repository, semantic, integrity, and integration invariants.
  Doctor {
    /// Verify a canonical Final Evaluation receipt by content identity.
    #[arg(long, value_name = "EVALUATION_ID")]
    receipt: Option<String>,
    #[arg(long)]
    json: bool,
  },
  /// Run the four-operation Model Context Protocol server over stdio.
  Mcp,
  /// Print the Tenet executable version.
  Version,
}

impl Command {
  pub(crate) fn json_requested(&self) -> bool {
    matches!(
      self,
      Self::Init { json: true, .. } | Self::Doctor { json: true, .. }
    )
  }
}
