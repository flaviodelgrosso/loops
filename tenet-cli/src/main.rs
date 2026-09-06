mod cli;
mod mcp;

use std::{process::ExitCode, sync::Arc};

use anyhow::Result;
use clap::Parser;
use tenet_application::{
  application::{InitializeRequest, Tenet},
  response::{ErrorResult, TenetError},
};

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
  let cli = Cli::parse();
  let json = cli.command.json_requested();
  if let Err(error) = run_command(cli) {
    report_error(error, json);
    return ExitCode::FAILURE;
  }
  ExitCode::SUCCESS
}

fn run_command(cli: Cli) -> Result<()> {
  let cwd = cli.cwd.unwrap_or(std::env::current_dir()?);
  let tenet = Tenet::new(
    cwd.clone(),
    Arc::new(tenet_workspace::LocalWorkspace),
    Arc::new(tenet_runner::LocalProcessRunner),
  );
  match cli.command {
    Command::Init { spec, json } => {
      let result = tenet.initialize(&InitializeRequest { spec_path: spec })?;
      if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
      } else {
        println!("initialized: {}", result.initialized);
        println!(
          "specification: {} ({})",
          result.spec_path, result.spec_digest
        );
        println!("skill: {}", result.skill_path);
      }
      Ok(())
    }
    Command::Doctor { json } => {
      let result = tenet.doctor()?;
      if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
      } else {
        for check in &result.checks {
          println!(
            "{}: {} — {}",
            if check.passed { "PASS" } else { "FAIL" },
            check.name,
            check.detail
          );
        }
      }
      if result.healthy {
        Ok(())
      } else {
        Err(TenetError::new("doctor_failed", "one or more doctor checks failed").into())
      }
    }
    Command::Mcp => mcp::run(cwd),
    Command::Version => {
      println!("tenet {}", env!("CARGO_PKG_VERSION"));
      Ok(())
    }
  }
}

fn report_error(error: anyhow::Error, json: bool) {
  let typed = error
    .chain()
    .find_map(|cause| cause.downcast_ref::<TenetError>().cloned())
    .unwrap_or_else(|| TenetError::new("internal_error", error.to_string()));
  let error: ErrorResult = typed.into();
  if json {
    match serde_json::to_string_pretty(&error) {
      Ok(encoded) => println!("{encoded}"),
      Err(encoding_error) => eprintln!(
        "error: {}; additionally failed to encode JSON error: {encoding_error}",
        error.message
      ),
    }
  } else {
    eprintln!("error: {}", error.message);
  }
}
