//! Enforces the six-crate architecture and its dependency direction.
//!
//! Required edges (a → b means "a may depend on b"):
//!
//! ```text
//! tenet-kernel     → tenet-domain
//! tenet-application→ tenet-domain, tenet-kernel
//! tenet-workspace  → tenet-domain, tenet-kernel, tenet-application
//! tenet-runner     → tenet-domain, tenet-kernel, tenet-application
//! tenet-cli        → any Tenet crate (composition root)
//! ```
//!
//! `tenet-domain` depends on no Tenet crate; `tenet-workspace` never depends
//! on `tenet-runner` and vice versa; nothing references the removed
//! `tenet-mcp` crate.

use std::{collections::BTreeSet, fs, path::Path};

use toml::Table;

const CRATES: [&str; 6] = [
  "tenet-domain",
  "tenet-kernel",
  "tenet-application",
  "tenet-workspace",
  "tenet-runner",
  "tenet-cli",
];

fn allowed_dependencies(crate_name: &str) -> BTreeSet<&'static str> {
  let allowed: &[&str] = match crate_name {
    "tenet-domain" => &[],
    "tenet-kernel" => &["tenet-domain"],
    "tenet-application" => &["tenet-domain", "tenet-kernel"],
    "tenet-workspace" | "tenet-runner" => &["tenet-domain", "tenet-kernel", "tenet-application"],
    "tenet-cli" => &CRATES[..],
    other => panic!("unexpected crate {other}"),
  };
  allowed.iter().copied().collect()
}

fn workspace_root() -> std::path::PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .expect("workspace root")
    .to_path_buf()
}

fn tenet_dependencies(manifest: &Table) -> BTreeSet<String> {
  let mut found = BTreeSet::new();
  for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
    let Some(table) = manifest.get(section).and_then(|value| value.as_table()) else {
      continue;
    };
    for name in table.keys() {
      if name.starts_with("tenet-") {
        found.insert(name.clone());
      }
    }
  }
  found
}

#[test]
fn workspace_contains_exactly_the_six_target_crates() {
  let root = workspace_root();
  let manifest: Table = fs::read_to_string(root.join("Cargo.toml"))
    .expect("root manifest")
    .parse()
    .expect("valid toml");
  let members = manifest
    .get("workspace")
    .and_then(|workspace| workspace.get("members"))
    .and_then(toml::Value::as_array)
    .expect("workspace members");
  let members = members
    .iter()
    .map(|member| member.as_str().expect("string member").to_owned())
    .collect::<BTreeSet<_>>();
  assert_eq!(
    members,
    CRATES.iter().map(|name| (*name).to_owned()).collect(),
    "workspace members must be exactly the six target crates"
  );
  for name in CRATES {
    assert!(
      root.join(name).join("Cargo.toml").is_file(),
      "{name} manifest exists"
    );
  }
  assert!(
    !root.join("tenet-mcp").exists(),
    "tenet-mcp must not exist as a crate"
  );
  assert!(
    !root.join("crates").exists(),
    "no intermediate crates/ directory"
  );
}

#[test]
fn every_crate_respects_the_dependency_direction() {
  let root = workspace_root();
  for name in CRATES {
    let path = root.join(name).join("Cargo.toml");
    let manifest: Table = fs::read_to_string(&path)
      .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
      .parse()
      .expect("valid toml");
    let actual = tenet_dependencies(&manifest);
    let allowed = allowed_dependencies(name);
    for dependency in &actual {
      assert!(
        allowed.contains(dependency.as_str()),
        "{name} must not depend on {dependency}; allowed: {allowed:?}"
      );
    }
  }
}

#[test]
fn kernel_application_workspace_and_runner_are_directly_wired() {
  let root = workspace_root();
  let required = [
    ("tenet-kernel", "tenet-domain"),
    ("tenet-application", "tenet-kernel"),
    ("tenet-workspace", "tenet-application"),
    ("tenet-runner", "tenet-application"),
    ("tenet-cli", "tenet-workspace"),
    ("tenet-cli", "tenet-runner"),
  ];
  for (crate_name, dependency) in required {
    let manifest: Table = fs::read_to_string(root.join(crate_name).join("Cargo.toml"))
      .expect("manifest")
      .parse()
      .expect("valid toml");
    assert!(
      tenet_dependencies(&manifest).contains(dependency),
      "{crate_name} should depend on {dependency}"
    );
  }
}
