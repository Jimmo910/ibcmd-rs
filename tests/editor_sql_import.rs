//! Clean-room source admission must fail before SQL is contacted.
//! Transaction/publication and real SQL acceptance are separate gates.
use ibcmd_rs::{
    file_import::Selection,
    mssql_source_change::SourceTraversalLimits,
    server::sql_import::{SourceAdmission, SourceLimits},
};
use std::{fs, path::Path};
#[path = "support/owned_scratch.rs"]
mod owned_scratch;
use owned_scratch::OwnedScratch;

const MODULE: &str = "CommonModules/Synthetic/Ext/Module.bsl";
fn fixture() -> OwnedScratch {
    let root = OwnedScratch::new("editor-sql-source");
    fs::create_dir_all(root.join("CommonModules/Synthetic/Ext")).unwrap();
    fs::write(
        root.join("CommonModules/Synthetic.xml"),
        b"<MetaDataObject/>",
    )
    .unwrap();
    fs::write(root.join(MODULE), b"// before\n").unwrap();
    root
}
fn selected() -> Selection {
    Selection {
        files: vec![MODULE.into()],
        ..Selection::default()
    }
}

#[test]
fn selection_and_verification_refuse_before_source_or_sql_io() {
    let absent = Path::new("nonexistent-editor-sql-admission-root");
    for selection in [
        Selection::default(),
        Selection {
            files: vec![MODULE.into()],
            changed: true,
            ..Selection::default()
        },
    ] {
        let error = SourceAdmission::prepare(absent, &selection, None, SourceLimits::default())
            .err()
            .unwrap();
        assert!(error.to_string().contains("exactly one"), "{error:#}");
    }
    for object in ["Configuration", "configuration", "Configuration.Synthetic"] {
        let selection = Selection {
            objects: vec![object.into()],
            ..Selection::default()
        };
        let error = SourceAdmission::prepare(absent, &selection, None, SourceLimits::default())
            .err()
            .unwrap();
        assert!(error.to_string().contains("Configuration"), "{error:#}");
    }
    let error = SourceAdmission::prepare(absent, &selected(), Some(false), SourceLimits::default())
        .err()
        .unwrap();
    assert!(error.to_string().contains("verification"), "{error:#}");
}

#[test]
fn admitted_bytes_are_immutable_and_changed_source_is_refused() {
    let root = fixture();
    let admitted =
        SourceAdmission::prepare(&root, &selected(), Some(true), SourceLimits::default()).unwrap();
    admitted.revalidate().unwrap();
    assert_eq!(admitted.bytes(MODULE).unwrap(), b"// before\n");
    fs::write(root.join(MODULE), b"// changed\n").unwrap();
    assert_eq!(admitted.bytes(MODULE).unwrap(), b"// before\n");
    assert!(admitted.revalidate().is_err());
}

#[test]
fn traversal_and_missing_or_aliased_files_refuse_without_sql() {
    let root = fixture();
    let limits = SourceLimits {
        traversal: SourceTraversalLimits {
            max_entries: 2,
            max_depth: 64,
        },
        ..SourceLimits::default()
    };
    assert!(SourceAdmission::prepare(&root, &selected(), None, limits).is_err());
    for file in [
        "../outside.bsl",
        "CommonModules/Synthetic/Ext/Missing.bsl",
        "Configuration.xml",
        "commonmodules/Synthetic/Ext/Module.bsl",
    ] {
        let selection = Selection {
            files: vec![file.into()],
            ..Selection::default()
        };
        assert!(
            SourceAdmission::prepare(&root, &selection, None, SourceLimits::default()).is_err(),
            "{file}"
        );
    }
}

#[test]
fn changed_mode_requires_synchronization_inventory_before_sql() {
    let root = fixture();
    let selection = Selection {
        changed: true,
        ..Selection::default()
    };
    let error = SourceAdmission::prepare(&root, &selection, None, SourceLimits::default())
        .err()
        .unwrap();
    assert!(format!("{error:#}").contains("ConfigDumpInfo.xml"));
}

#[cfg(unix)]
#[test]
fn source_links_are_refused_before_sql() {
    let root = fixture();
    std::os::unix::fs::symlink(root.join(MODULE), root.join("alias.bsl")).unwrap();
    assert!(SourceAdmission::prepare(&root, &selected(), None, SourceLimits::default()).is_err());
}

mod physical_readset_contracts {
    use ibcmd_rs::{
        mssql_live_continue::DatabaseIdentity,
        server::sql_import::readset::{Builder, Check, Input, Key, Limits, Part, Table},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    fn identity() -> DatabaseIdentity {
        DatabaseIdentity {
            server: "synthetic-server".into(),
            database: "synthetic-database".into(),
            database_guid: uuid::Uuid::from_u128(1).to_string(),
            family_guid: uuid::Uuid::from_u128(2).to_string(),
            recovery_fork: uuid::Uuid::from_u128(3).to_string(),
        }
    }
    fn row(name: &str, part: u32, size: u64, bytes: &[u8], header: &[u8]) -> Input {
        Input {
            key: Key {
                file_name: name.into(),
                part_no: part,
            },
            part: Part {
                data_size: size,
                header: header.to_vec(),
                binary: Arc::new(bytes.to_vec()),
            },
        }
    }
    fn builder() -> Builder {
        Builder::new(identity(), [1; 32], Limits::default(), Arc::new(|| Ok(()))).unwrap()
    }
    fn baseline() -> ibcmd_rs::server::sql_import::readset::Readset {
        let mut b = builder();
        b.push(
            Table::Config,
            row("selected", 0, 3, b"old", b"config-header"),
        )
        .unwrap();
        b.push(
            Table::Config,
            row("unrelated", 0, 3, b"old", b"config-header"),
        )
        .unwrap();
        b.push(
            Table::ConfigSave,
            row("neighbor", 0, 4, b"aa", b"stage-header-0"),
        )
        .unwrap();
        b.push(
            Table::ConfigSave,
            row("neighbor", 1, 4, b"bb", b"stage-header-1"),
        )
        .unwrap();
        b.finish().unwrap()
    }
    #[test]
    fn every_unselected_staged_part_and_header_is_preserved() {
        let original = baseline();
        let merged = original
            .with_selected_stage([
                row("selected", 0, 4, b"ab", b"new-0"),
                row("selected", 1, 4, b"cd", b"new-1"),
            ])
            .unwrap();
        assert!(
            original
                .parts(Table::Config)
                .eq(merged.parts(Table::Config))
        );
        assert!(
            original.parts(Table::ConfigSave).eq(merged
                .parts(Table::ConfigSave)
                .filter(|(key, _)| key.file_name == "neighbor"))
        );
        assert_ne!(original.digest().unwrap(), merged.digest().unwrap());
        assert!(
            original
                .with_selected_stage([row("new", 0, 1, b"x", b"header")])
                .is_err()
        );
    }
    #[test]
    fn shrinking_selected_stage_removes_old_tail_and_preserves_neighbor() {
        let original = baseline()
            .with_selected_stage([
                row("selected", 0, 4, b"ab", b"old-0"),
                row("selected", 1, 4, b"cd", b"old-1"),
            ])
            .unwrap();
        let replacement = row("selected", 0, 1, b"x", b"new-header");
        let expected = replacement.part.clone();
        let merged = original.with_selected_stage([replacement]).unwrap();
        let selected = merged
            .parts(Table::ConfigSave)
            .filter(|(key, _)| key.file_name == "selected")
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].0.part_no, 0);
        assert_eq!(selected[0].1, &expected);
        assert!(
            original
                .parts(Table::ConfigSave)
                .filter(|(key, _)| key.file_name == "neighbor")
                .eq(merged
                    .parts(Table::ConfigSave)
                    .filter(|(key, _)| key.file_name == "neighbor"))
        );
        assert!(
            original
                .parts(Table::Config)
                .eq(merged.parts(Table::Config))
        );
        assert!(original.with_selected_stage([]).is_err());
    }
    #[test]
    fn individually_admissible_replacements_cannot_exceed_merged_budget() {
        let limits = Limits {
            max_payload_bytes: 4,
            ..Limits::default()
        };
        let mut b = Builder::new(identity(), [1; 32], limits, Arc::new(|| Ok(()))).unwrap();
        b.push(Table::Config, row("selected", 0, 2, b"ab", b"h"))
            .unwrap();
        b.push(Table::ConfigSave, row("neighbor", 0, 2, b"cd", b"h"))
            .unwrap();
        let original = b.finish().unwrap();
        assert!(
            original
                .with_selected_stage([row("selected", 0, 1, b"x", b"h")])
                .is_err()
        );
        assert_eq!(original.parts(Table::ConfigSave).count(), 1);
        assert_eq!(
            original
                .parts(Table::ConfigSave)
                .next()
                .unwrap()
                .1
                .binary
                .as_slice(),
            b"cd"
        );
    }
    #[test]
    fn identity_schema_unrelated_rows_headers_additions_and_deletions_change_binding() {
        let original = baseline().digest().unwrap();
        for variant in 0..6 {
            let mut id = identity();
            if variant == 0 {
                id.recovery_fork = uuid::Uuid::from_u128(4).to_string();
            }
            let mut b = Builder::new(
                id,
                if variant == 1 { [2; 32] } else { [1; 32] },
                Limits::default(),
                Arc::new(|| Ok(())),
            )
            .unwrap();
            b.push(
                Table::Config,
                row("selected", 0, 3, b"old", b"config-header"),
            )
            .unwrap();
            if variant != 5 {
                b.push(
                    Table::Config,
                    row(
                        "unrelated",
                        0,
                        3,
                        if variant == 2 { b"new" } else { b"old" },
                        b"config-header",
                    ),
                )
                .unwrap();
            }
            b.push(
                Table::ConfigSave,
                row(
                    "neighbor",
                    0,
                    4,
                    b"aa",
                    if variant == 3 {
                        b"changed-header"
                    } else {
                        b"stage-header-0"
                    },
                ),
            )
            .unwrap();
            b.push(
                Table::ConfigSave,
                row("neighbor", 1, 4, b"bb", b"stage-header-1"),
            )
            .unwrap();
            if variant == 4 {
                b.push(Table::ConfigSave, row("additional", 0, 1, b"x", b"header"))
                    .unwrap();
            }
            assert_ne!(
                original,
                b.finish().unwrap().digest().unwrap(),
                "variant {variant}"
            );
        }
    }
    #[test]
    fn duplicates_and_budget_errors_poison_the_whole_readset() {
        let mut b = builder();
        b.push(Table::Config, row("selected", 0, 1, b"a", b"h"))
            .unwrap();
        assert!(
            b.push(Table::Config, row("selected", 0, 1, b"a", b"h"))
                .is_err()
        );
        assert!(b.finish().is_err());
        let mut b = Builder::new(
            identity(),
            [1; 32],
            Limits {
                max_payload_bytes: 1,
                ..Limits::default()
            },
            Arc::new(|| Ok(())),
        )
        .unwrap();
        assert!(
            b.push(Table::Config, row("selected", 0, 2, b"ab", b"h"))
                .is_err()
        );
        assert!(b.finish().is_err());
    }
    #[test]
    fn part_count_part_size_name_and_identity_metadata_budgets_are_enforced() {
        for limits in [
            Limits {
                max_parts: 1,
                ..Limits::default()
            },
            Limits {
                max_part_bytes: 1,
                ..Limits::default()
            },
            Limits {
                max_name_units: 1,
                ..Limits::default()
            },
        ] {
            let mut b = Builder::new(identity(), [1; 32], limits, Arc::new(|| Ok(()))).unwrap();
            if limits.max_parts == 1 {
                b.push(Table::Config, row("a", 0, 1, b"a", b"h")).unwrap();
            }
            assert!(b.push(Table::Config, row("xx", 0, 2, b"ab", b"h")).is_err());
            assert!(b.finish().is_err());
        }
        assert!(
            Builder::new(
                identity(),
                [1; 32],
                Limits {
                    max_metadata_bytes: 1,
                    ..Limits::default()
                },
                Arc::new(|| Ok(()))
            )
            .is_err()
        );
    }

    #[test]
    fn part_gaps_inconsistent_sizes_and_truncated_payloads_are_refused() {
        for variant in 0..3 {
            let mut b = builder();
            b.push(Table::Config, row("selected", 0, 4, b"aa", b"h"))
                .unwrap();
            if variant != 2 {
                b.push(
                    Table::Config,
                    row(
                        "selected",
                        if variant == 0 { 2 } else { 1 },
                        if variant == 1 { 5 } else { 4 },
                        b"bb",
                        b"h",
                    ),
                )
                .unwrap();
            }
            assert!(b.finish().is_err());
        }
    }
    #[test]
    fn cancellation_remains_typed_after_capture() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let observed = cancelled.clone();
        let check: Check = Arc::new(move || {
            if observed.load(Ordering::Relaxed) {
                Err(ibcmd_rs::cancel::Cancelled.into())
            } else {
                Ok(())
            }
        });
        let mut b = Builder::new(identity(), [1; 32], Limits::default(), check).unwrap();
        b.push(Table::Config, row("selected", 0, 3, b"old", b"header"))
            .unwrap();
        let snapshot = b.finish().unwrap();
        cancelled.store(true, Ordering::Relaxed);
        assert!(ibcmd_rs::cancel::is_cancelled(
            &snapshot.digest().unwrap_err()
        ));
    }
}
