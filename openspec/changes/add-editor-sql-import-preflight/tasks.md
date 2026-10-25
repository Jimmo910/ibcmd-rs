# Tasks

- [x] 1.1 Inspect current #382, merged #481 and verified current master.
- [x] 1.2 Record bounded SQL acceptance, immutable-plan and transaction contract.
- [ ] 2.1 Add clean-room fail-before-write regression cases (#382).
- [ ] 2.2 Implement bounded SQL preparation using existing compiler/verifier.
- [ ] 2.3 Implement same-session identity/readset/ConfigSave CAS publication,
  retaining all unselected staging; keep capability closed until verified.
- [ ] 2.4 Coordinate server token routing without changing FileDb behavior.
- [ ] 3.1 Run allocated focused tests and independent review.
- [ ] 3.2 Through the authorized operator, verify disposable SQL module/form
  stage -> native apply -> export and unchanged neighbour/staging.
- [ ] 3.3 Run required current-head quality/CI gates; publish honest evidence.

## Delivered local foundations (not SQL acceptance)

- [x] F.1 Mechanically extract neutral compiler planner with FileDb API preserved;
  donor 59a4a8f4, independently reviewed; original 2+14+6 narrow tests passed.
- [x] F.2 Bounded original-source admission and physical readset arithmetic;
  checkpoint 52ea775c, 14 synthetic integration tests passed.
- [x] F.3 Single-request read-only snapshot adapter and recording contract;
  23 combined integration tests and library check passed after reviewed fixes.
- [ ] F.4 Measure exact SQL/schema/header profile and prove actual acquisition
  locking, empty-resultset numbering, interruption and concurrent-writer controls.
- [x] F.5 Owned structural body-coverage invariant and omission/expansion/budget/
  cancellation controls: 27 combined tests and library check passed; not wired
  to shared effective-view policy or SQL routing and not an ownership proof.
- [x] F.6 Opt-in raw old/new export evidence, bounded collision/selection binding
  checks and real diagnostic consumer; fresh static review, owned-warning-free
  library check, 15 planner +14 FileDb +29 SQL-contract tests passed. Tightened
  refusal oracle passed one exact test; all fixtures and prior failures retained.
- [ ] F.7 Independent complete owner/kind/full-parent/selected-role witnesses,
  origin-aware versions authority and full readset-preserving SQL transaction.

- [x] F.8 Direct snapshot-only existing top-level CommonModule compiler, exact
  registered body-key selection and SQL-origin body-only versions guard; 10
  focused tests and production lib-check passed. No observer dependency or SQL
  route/publisher. See evidence/selected-module-snapshot.md; this is not F.7 or
  completion of SQL preparation/publication.

- [x] F.9 Document-only root-cohort Form module compiler from immutable snapshots;
  exact complete declared owner/child identity checks and unchanged non-module
  body state. 17 selected-module/Form tests and integrated lib-check passed.
  Form.xml and unexamined owners remain unsupported; see
  evidence/form-module-and-cas-contract.md.
- [x] F.10 Pure exact full physical-readset comparison plus 3 mutation/cancellation
  tests; concrete dedicated-session/rollback/COMMIT-unknown contract reviewed.
  This does not implement transactional SQL CAS/publication or complete F.7.

- [x] F.11 Minimal dedicated-session backend seam, unsupported default, owned MSSQL
  connection, poison/terminal rollback guards; 4 pure state/default-refusal tests
  passed and source review cleared at bbb1929; real-session proof remains open.
  The later library transaction primitive is tracked separately below.

- [x] F.12 Library held-lock CAS/DML and explicit transaction outcomes with 11
  recording tests, 29 existing SQL-contract regressions and lib-check passed;
  source review CLEAR. No real SQL session/write or user route was executed.
  See evidence/held-lock-cas-recording.md. Real lock/native writer admission and
  end-to-end acceptance remain unfinished 2.3/3.2, not a completed SQL capability.


- [x] F.13 Actual exact-a666 Windows held-capture/rollback/closure, experimental
  selected CommonModule staging, and native 8.3.27.2342 apply/full-export passed.
  Active CONFIG independently matched staged body/versions; all unselected
  CONFIG rows and four XML files were exact, CDI only selected version changed.
  This is the one-module public-fixture slice, not completion of 2.3/3.2 or
  general writer admission. Nonempty neighbour staging, native FormModule,
  live negative controls and guarded editor SQL routing remain open.
