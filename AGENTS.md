# AGENTS.md

## Product boundary

Tenet is an agent-neutral, CLI-first completion authority for exact content identities.

The coding agent owns investigation, planning, editing, tests, and blocker responses. Tenet owns immutable authority state, admitted contract semantics, verifier observations and provenance, Candidate capture, and deterministic completion derivation.

No agent statement, model output, generated Skill, mutable ref, or verifier exit code is itself a completion decision. Only `tenet_verify` may return protocol-level `DONE`, derived by the kernel from one Final Evaluation.

## Final protocol

Expose exactly four completion-domain MCP operations:

```text
tenet_context
tenet_authority_submit
tenet_requirement_check
tenet_verify
```

The initial CLI surface is exactly `init`, `doctor`, `mcp`, and `version`. Do not reintroduce public propose/approve/seal/select/capture/gate workflows.

`tenet_context` derives phase from persisted facts. Never persist workflow phase. `COMPLETED` requires a successful Final Evaluation for the active Admission and Authority whose Candidate equals a fresh current capture.

`tenet_authority_submit` implements `PROPOSAL`, `RECONCILIATION`, `CLARIFICATION`, and `ADMISSION`. Every transition binds exact identities. Clarification never admits. Admission validates the complete exact chain.

`tenet_requirement_check` captures one Candidate, reruns all verifiers for one Requirement using a fresh Candidate view per verifier, persists a Requirement-scoped Evaluation, and updates its ref. It cannot establish terminal completion.

`tenet_verify` captures the final Candidate once, reruns every required verifier with a fresh Candidate view per verifier, persists one Final Evaluation, and delegates all completion semantics to the kernel. Requirement-check runs are never promoted. A successful Final `EvaluationId` is the `LOCAL_V1` receipt identity; do not add another receipt type.

After a successful Final Evaluation, recapture current content. If the Candidate changed, preserve the historical Evaluation but return `INCONCLUSIVE` with `CANDIDATE_CHANGED_DURING_VERIFICATION`, the verified Candidate, and the current Candidate. Never apply historical evidence to the new state.

## Authority and Candidate identities

Authority and Candidate are distinct typed, content-addressed identities. The active Authority is derived only by loading `.tenet/refs/active-admission` and validating the referenced immutable Admission, Proposal, Reconciliation, Authority, and SpecSnapshot chain.

A Proposal is not Admission. A mutable ref has no authority independent of its referenced immutable object. Reconciliation for one Proposal cannot authorize another. Admission for one Authority cannot authorize another.

Candidate capture uses policy from the admitted immutable Authority, never live mutable policy. `tenet:candidate-semantics:v1` and all Tenet-owned format versions remain version `1` during the unreleased MVP. Unknown versions and semantics fail closed; do not add migrations or compatibility branches.

## Evidence and completion

Every `VerifierRun` binds exact Authority and Candidate subjects, captured observation, execution context, Runner semantics, assurance, and provenance. Final Evaluation contains exactly one run for every required verifier and no duplicates or extras.

Completion must fail closed for missing, stale, cross-subject, duplicate, inadmissible, contradictory, inconclusive, infrastructure-failed, or unknown-semantic evidence. Assurance requirements participate in completion. `LOCAL_V1` cannot satisfy `Protected`.

Candidate-controlled evidence is admissible only when explicitly permitted by the Authority's CompletionContract. Verifier definitions come from immutable Authority, while Candidate inputs come from the exact captured Candidate.

## Trust distinctions

Never collapse these boundaries:

- `LOCAL_V1` is not same-user tamper resistance.
- `AuthorityBound` is not independent authorship.
- fresh materialization is not sandboxing.
- content addressing is not writer authentication.
- MCP user input is not cryptographic human identity.
- verifier `Pass` is not task completion.

Do not add passwords, HMACs, keychains, signatures, privileged services, or mandatory containers to imply guarantees the current same-user local boundary does not provide.

## Persistence

Runtime state is repository-contained:

```text
.tenet/
├── format
├── .gitignore
├── objects/
├── blobs/
├── refs/
│   ├── proposal
│   ├── reconciliation
│   ├── active-admission
│   ├── final
│   └── requirements/
├── tmp/
└── lock
```

Objects and blobs are immutable and content-addressed. Refs are mutable navigation only. All writes must remain beneath the repository root and reject symlink/path escape. Phase is derived, never stored.

`tenet doctor` validates repository root, `SPEC.md`, format, object/blob/ref integrity, active Admission chain, supported semantic versions, repository-write scope, and integration consistency.

## Domain and validation

Rust domain types are the source of truth. Derive serialization and JSON Schema where practical. Keep layers distinct:

```text
syntax → schema/Serde → domain invariants → repository/runtime invariants
```

Use semantic ID newtypes where identity confusion matters. Use `thiserror` for distinguishable domain errors and `anyhow` at CLI/I/O boundaries. Avoid `unwrap` and `expect` in production code.

Breaking changes are allowed during MVP. Make clean cutovers: update every caller, test, fixture, generated integration, and active document; remove obsolete variants and competing paths. All Tenet-owned schema and format versions remain `1` until public release.

## Six-crate architecture

The workspace contains exactly:

- `tenet-domain`: semantic vocabulary only;
- `tenet-kernel`: pure deterministic identity, admission, evidence, completion, and phase semantics; depends only on domain;
- `tenet-application`: use cases and repository/runner ports; no filesystem, process, or persistence work;
- `tenet-workspace`: repository, object/blob/ref persistence, capture, and materialization;
- `tenet-runner`: structured process execution, timeout, bounded output, and provenance;
- `tenet-cli`: CLI and MCP composition root.

Dependency direction is `domain ← kernel ← application`; workspace and runner implement application ports and never depend on each other; CLI composes all layers. Delivery code contains no completion logic.

Prefer existing files and direct primitives. Do not add provider integrations, model runtimes, plugin systems, databases, generic rule engines, or speculative traits. Structured verifier commands use explicit argv, cwd, environment, timeout, and bounded output without an implicit shell.

## Testing

Architectural changes require deterministic offline adversarial tests. Preserve coverage for:

- producer assertions cannot create `DONE`;
- candidate-controlled verifier trust requires explicit Authority policy;
- evidence cannot transfer across Candidate or Authority identities;
- reconciliation and admission cannot transfer across identities;
- missing or duplicate runs cannot hide missing evidence;
- `LOCAL_V1` cannot satisfy `Protected`;
- every verifier gets a fresh Candidate view;
- mutation during final verification cannot produce `DONE` for the new state;
- unknown semantic versions fail closed;
- CLI and MCP cannot redefine kernel completion semantics.

Before completion, run:

```bash
make ci
```

This checks formatting, compilation, Clippy with warnings denied, and all tests.

## Working style

1. Inspect domain types, kernel semantics, application flow, adapters, and callers before editing.
2. Preserve the Admission/Authority/Candidate/Evaluation identity split at every interface.
3. Reuse established patterns; keep changes materially small.
4. Update all affected callers and tests in one clean cutover.
5. Verify focused adversarial behavior, then run full CI.
6. Report any unverifiable invariant; never weaken completion semantics to obtain green output.
