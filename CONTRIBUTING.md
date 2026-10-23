# Contributing to ibcmd-rs

Most of ibcmd-rs is written with coding agents, and contributions made with
them are welcome. This page is the contract for people and agents alike: how
work is taken, verified, reviewed and merged. Agents get the same rules in
short form in [`AGENTS.md`](AGENTS.md).

## Ground rules

- **One issue, one branch, one pull request.** Larger or unclear work starts
  as an issue; milestone work is tracked on the
  [board](https://github.com/users/Untru/projects/9).
- **Evidence over claims.** A change is done when its acceptance criteria hold
  and the PR shows how anyone can check it: commands, results, measurements.
- **Fail closed.** An unknown profile, an unexpected opaque record or an
  unclassified loss is an error before anything is published, never a guess.
- **Clean-room data only.** Never commit private `.cf`/`.1CD` files, native
  export trees, raw blobs, application XML/BSL, credentials or client paths.
- **Disposable copies only.** Commands that write to an infobase run only
  against copies made for the purpose.

## Setup

- Rust through rustup. CI pins **1.95.0**
  (`rustup toolchain install 1.95.0 -c clippy,rustfmt`); a newer stable works
  for daily use, and `cargo +1.95.0 …` settles a disagreement with CI.
- For the full local gate (`cargo xtask ci`): PowerShell 7 (`pwsh`),
  Python 3 and Node.js (`npx`). A missing tool turns its steps into a
  reported `SKIP`, never into a silent pass.
- For parity work: an installed 1C platform with `ibcmd`. SQL Server and
  external corpora are needed only by opt-in suites (`docs/testing.md`).
- The workspace is large (about 720k lines of Rust). Give every parallel
  agent its own worktree; two cargo commands in one target directory wait for
  each other.

## The verification loop

One entry point, `cargo xtask`, runs the same commands for people, agents and
agent hooks. It prints one line per step and the tail of a failing step; full
logs are in `target/xtask/`.

| Tier | Command | Runs | When |
| --- | --- | --- | --- |
| Edit | `cargo fmt --all` (agent hook: per file) | rustfmt | after every edit |
| Scope | `cargo xtask affected` | the test commands for your diff | before testing |
| Inner | `cargo xtask check` | format, clippy `-D warnings` on `crates/*`, compile check of the root package, tests of the changed crates and root suites | after every step |
| Gate | `cargo xtask ci` | the required merge checks that run locally: OpenSpec (strict), corpus and policy guards, guard self-tests, all-target check, workspace tests, root integration suites | before a PR leaves draft |
| CI | `Offline E2E` workflow | the gate on five platforms, plus the release build, native micro-corpus parity, SBOM and release audit | every push |

Measured on a 4-core machine with a warm dependency cache: `check` takes two
to three minutes on a fresh checkout and seconds when nothing changed; `ci`
takes 20–25 minutes, most of it compiling and running the root integration
suites.

Rules of the loop:

- A bug fix or new behaviour comes with a test that fails without it. Watch it
  fail first.
- A failure is pre-existing only when it fails the same way on the merge base
  (`git merge-base HEAD origin/master`); say so in the PR with both commands.
- Never `#[ignore]`, relabel, feature-gate or delete a test to get green.
- Expected outputs change only when the new output is proven right (native
  evidence or an EDT writer rule), never because your code now emits it.
- Plain `cargo test --workspace` is not the gate: the legacy root unit
  harness has known failures. Test boundaries are in `docs/testing.md`.

## Workflow

1. **Take an issue.** Set its board Status to In progress and record who works
   on it (details in `AGENTS.md`, Tracking).
2. **Branch.** Agents use `agent/issue-<N>-<slug>`; people may name branches
   freely. Start from `origin/master`.
3. **Specify.** A new capability (command, format, conversion route, gate)
   starts with an OpenSpec change in `openspec/changes/` validated with
   `--strict`. Bug fixes, refactors and test work do not need one.
4. **Implement** in small steps, each followed by the narrowest test.
5. **Verify** with `cargo xtask check`, then `cargo xtask ci`.
6. **Review.** Before the PR leaves draft, an independent reviewer that did
   not write the change reads the diff in a fresh context (for agents: the
   `critic` subagent, or a new session with its instructions). On the PR,
   CodeRabbit and the maintainer review.
7. **Merge.** The maintainer merges with a merge commit. A release tag waits
   for the milestone check in `docs/release-criteria.md`.

## Commits and pull requests

- Subject `type(scope): summary` (`feat(filedb): …`, `fix(xml): …`), with
  `#N` when it belongs to an issue. The body explains why.
- The PR description follows the template: **What**, **Why**, **Evidence**,
  then `Closes #N` (or `Refs #N` while acceptance stays open).
- Keep PRs reviewable: separate mechanical moves from behaviour changes, and
  do not mix unrelated fixes into an issue's PR.
- A PR stays draft until `cargo xtask ci` passes and the independent review
  approves.
- New dependencies change the SBOM, licences and the release audit; propose
  them in the issue first.

## Using AI agents

**Policy.**

- Agents may take issues, push branches and open draft PRs. A named person,
  the operator who started the agent, answers for every PR and every claim in
  it, and must be able to explain the change in review. "The agent did it" is
  not an answer.
- Disclose: commits keep the trailer the tool adds (Claude Code's
  `Co-Authored-By: Claude …`) or carry `Assisted-by: <tool>:<model>`; the PR
  names the tool and what it did.
- Agents do not merge PRs, create tags or close issues without an evidence
  comment. They update the board only for the issue they work on.
- Descriptions, comments and commit messages state facts from real runs. They
  describe the code, never the agent session ("as discussed", "the user
  asked").

**What each tool reads.**

| Tool | Instructions | Skills | Checks |
| --- | --- | --- | --- |
| Claude Code | `CLAUDE.md`, which imports `AGENTS.md` | `.claude/skills` (stubs pointing to `.agents/skills`) | hooks in `.claude/settings.json`; `critic` subagent |
| Codex | `AGENTS.md` | `.agents/skills` | run `cargo xtask check` yourself |
| Gemini CLI | `AGENTS.md` (through `.gemini/settings.json`) | `.agents/skills` | run `cargo xtask check` yourself |
| Copilot agent, Cursor | `AGENTS.md` | `.agents/skills` | run `cargo xtask check` yourself |

The repository skills are `verify-change` (tests, baseline, evidence),
`review-change` (independent review and PR hand-off), `native-parity`
(comparison with native `ibcmd`, evidence and fixtures) and
`openspec-change`. Each `.claude/skills/<name>/SKILL.md` repeats the name and
description of the shared skill and points to it; the repository refuses
symbolic links (`tools/validate-edt-corpus.ps1`), so change a description in
both places.

**Recommended additions** (installed per person, not committed):

- `rust-analyzer-lsp` for Claude Code:
  `rustup component add rust-analyzer`, then
  `claude plugin install rust-analyzer-lsp@claude-plugins-official --scope project`.
  It gives diagnostics after each edit and definition/reference lookup
  without reading the 40k-line files.
- Built-in Claude Code commands: `/code-review` on the diff before the
  critic, `/security-review` for SQL, TLS, authentication and file-locking
  code.
- [superpowers](https://github.com/obra/superpowers), already used for the
  plans in `docs/superpowers/`: its test-driven-development,
  systematic-debugging and verification-before-completion skills follow the
  loop above.
- Microsoft's [Pragmatic Rust Guidelines](https://microsoft.github.io/rust-guidelines/agents/all.txt)
  (agent edition, about 34k tokens) as reference material for reviewing new
  crates. Do not load it into every session.

## Maintaining the agent instructions

A 2026 evaluation of agent context files found that they do not generally
raise task success, add over 20% to inference cost, and that agents do follow
their instructions; repository overviews did not help, non-standard practices
did ([Gloaguen et al.](https://arxiv.org/abs/2602.11988)). Compliance with
written rules also drops as a session goes on
([McMillan](https://arxiv.org/abs/2605.10039)). So the file holds only what
an agent cannot infer, and what must hold is enforced by tools.

- `AGENTS.md` stays under 200 lines, is written by hand and holds traps, not
  maps. A new rule must be non-obvious, recurring and specific enough to act
  on. Agents propose rules in their PR description and do not edit the file
  in passing.
- Procedures go into skills under `.agents/skills/`; each states when to use
  it in its `description` and stays under 500 lines.
- Prefer a check over a sentence: a hook, a guard script, a test or an
  `xtask` step.
- `cargo xtask ci` mirrors `.github/workflows/offline-e2e.yml` and reads its
  OpenSpec list from there; change the workflow and `xtask/src/main.rs`
  together.
