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
