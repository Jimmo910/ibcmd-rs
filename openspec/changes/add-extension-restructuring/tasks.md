# Tasks

- [x] 1. Measure native extension schema transitions on 8.3.27.2214 and 8.5.1.1150, capture synthetic before/after schema and data (#582).
- [x] 2. Add platform-proven generated Coverage update cases and demonstrate the existing refusal before changing its guard (#582).
- [x] 3. Implement portable extension object/schema planning with measured DBNames/SchemaStorage/DDL rules and preservation tests (#582).
- [x] 4. Extend the XML/CFE update compiler to represent new own data objects and accompanying metadata/version edits faithfully (#582).
- [x] 5. Integrate schema planning with extension import/apply in an atomic guarded transaction, including rollback, no-op and concurrency paths (#582).
- [x] 6. Verify native-vs-own twins on both builds, existing row/reference preservation, fresh-session new-object writes/reads and subsequent native apply (#582).
- [ ] 7. Publish/pin the generated corpus, complete targeted tests, check/ci, independent review and open the PR with evidence (#582).
