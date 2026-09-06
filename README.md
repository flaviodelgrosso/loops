# Tenet

Tenet is an agent-neutral, CLI-first completion authority for exact content identities. It decides one claim:

> The exact Candidate identified by `CandidateId C` satisfies the completion contract carried by the exact Authority identified by `AuthorityId A`, under the exact active `AdmissionId`.

Tenet persists immutable objects and derives workflow and completion state. Agent statements, mutable refs, verifier exit codes, and generated integrations do not decide completion.

## CLI

```text
tenet init [--spec PATH] [--json]
tenet doctor [--json]
tenet mcp
tenet version
```

`tenet init` creates repository-contained state, a starter `SPEC.md` when needed, MCP configuration, and the Tenet Skill. `tenet doctor` validates repository root discovery, `SPEC.md`, repository format, object/blob/ref integrity, the active Admission chain, supported semantic versions, repository-write scope, and integration consistency.

## Four-operation protocol

The MCP completion-domain surface is exactly:

```text
tenet_context
tenet_authority_submit
tenet_requirement_check
tenet_verify
```

### `tenet_context`

Call this first. It derives, rather than persists:

- phase;
- active `AdmissionId`, `AuthorityId`, and `CompletionPolicyId`;
- current `CandidateId` when available;
- Requirement-check status;
- the next action.

Supported phases are `SPEC_REQUIRED`, `AUTHORITY_REQUIRED`, `AUTHORITY_RECONCILIATION`, `AUTHORITY_CLARIFICATION`, `AUTHORITY_ADMISSION`, `AUTHORITY_STALE`, `INCOMPATIBLE`, `IMPLEMENTATION`, and `COMPLETED`.

`COMPLETED` requires a successful Final Evaluation bound to the active Admission and Authority, kernel state `satisfied`, and a freshly captured current Candidate equal to the Evaluation Candidate.

### `tenet_authority_submit`

Submit one exact lifecycle stage:

```text
PROPOSAL → RECONCILIATION → CLARIFICATION (when needed) → ADMISSION
```

A Proposal captures the specification, `CompletionContractV1`, policy, and authority-owned verifier material into an immutable Authority. Reconciliation binds one exact Proposal. Clarification records information without admitting anything. Admission binds the exact Proposal, Reconciliation, and Authority. A ref has no authority independent of the referenced immutable object and validated chain.

Example request shape:

```json
{
  "submission": {
    "stage": "ADMISSION",
    "proposalId": "sha256:…",
    "reconciliationId": "sha256:…",
    "authorityId": "sha256:…"
  }
}
```

### `tenet_requirement_check`

A Requirement check:

1. loads the active Admission and derives its Authority;
2. captures current Candidate `C`;
3. runs every verifier for one Requirement;
4. gives every verifier a fresh materialization of `C` and fresh scratch directory;
5. persists one Requirement-scoped Evaluation;
6. derives the Requirement result in the kernel;
7. updates the Requirement ref.

This evidence is Candidate-specific development feedback. It never becomes Final evidence and this operation cannot return protocol-level `DONE`.

### `tenet_verify`

Final verification:

1. loads the exact active Admission and Authority;
2. requires supported `CompletionPolicyV1` and Runner/Candidate semantics;
3. captures `Cfinal` once;
4. reruns every required verifier, each against a fresh materialization of `Cfinal`;
5. persists one Final Evaluation containing the exact subject-bound run set;
6. derives all Criteria, Requirements, and the Authority outcome in the kernel.

Only `tenet_verify` can return `DONE`. A successful response identifies `AdmissionId`, `AuthorityId`, `CandidateId`, and `EvaluationId`. The successful Final `EvaluationId` is the `LOCAL_V1` receipt identity; there is no separate receipt object.

After a successful Evaluation for `C1`, Tenet captures the working tree again. If it is now `C2`, Tenet preserves the successful historical Evaluation for `C1` but returns `INCONCLUSIVE`, reason `CANDIDATE_CHANGED_DURING_VERIFICATION`, and both Candidate identities. Evidence for `C1` is never implied to cover `C2`.

## Persistence

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

Objects and blobs are addressed by SHA-256 content identity. Refs are mutable navigation pointers only. `.tenet/tmp` and `.tenet/lock` are disposable; workflow phase is not stored.

`tenet:candidate-semantics:v1` identifies a sorted manifest of normalized repository-relative regular-file paths, content IDs, and executable bits. `.tenet/**` and repository metadata are excluded from Candidate capture. Missing, corrupt, noncanonical, unknown-version, or unknown-semantics content fails closed.

## Completion and evidence policy

`CompletionPolicyV1` requires the exact verifier set for the Evaluation scope. Every run binds exact Admission-derived Authority and Candidate subjects. Duplicate, missing, extra, cross-Authority, and cross-Candidate runs are rejected. Assurance and evidence-control requirements participate in every Criterion result. Candidate-controlled verification is admissible only when the Authority contract explicitly permits it.

The local runner uses structured argv, typed Candidate/Authority/scratch paths, explicit environment inheritance, timeouts, bounded output, `RunnerSemanticsV1`, and `LOCAL_V1`. It invokes no implicit shell.

## Trust boundaries

These distinctions are mandatory:

- **`LOCAL_V1` ≠ same-user tamper resistance.** A same-user process can affect local execution; `LOCAL_V1` makes no stronger claim.
- **`AuthorityBound` ≠ independent authorship.** Binding verifier material to Authority identifies content; it does not prove who wrote it.
- **fresh materialization ≠ sandboxing.** Each verifier gets a pristine view, not an isolation or containment guarantee.
- **content addressing ≠ writer authentication.** A digest identifies bytes; it does not authenticate their producer.
- **MCP user input ≠ cryptographic human identity.** Admission is an explicit workflow boundary, not a signature scheme.
- **verifier `Pass` ≠ task completion.** Only deterministic kernel evaluation of the full admitted Final Evaluation can yield `DONE`.

## Architecture

The workspace has exactly six crates:

- `tenet-domain`: semantic types and errors;
- `tenet-kernel`: pure identity, admission, policy, phase, and completion derivation;
- `tenet-application`: protocol use cases and infrastructure ports;
- `tenet-workspace`: repository-contained persistence and materialization;
- `tenet-runner`: process execution and provenance;
- `tenet-cli`: CLI and MCP composition root.

Dependency direction is enforced by tests: `domain ← kernel ← application`, with `workspace` and `runner` implementing application ports and `cli` composing them.

## Development

```bash
make ci
```

This checks formatting, compilation, Clippy with warnings denied, and all deterministic offline tests.
