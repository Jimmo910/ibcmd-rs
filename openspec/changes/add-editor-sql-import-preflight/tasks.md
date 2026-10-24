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
