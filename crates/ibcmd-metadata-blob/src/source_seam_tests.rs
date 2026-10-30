//! The source context answers through [`MetadataSource`] exactly as through
//! its own methods: the same values, the same memoised state, and the same
//! error values with the same messages.

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;

use super::{HelpReference, MetadataSource, MetadataSourceContext, SourceBytes};

const CONFIGURATION: &str = "11111111-1111-4111-8111-111111111111";
const STYLE_ITEM: &str = "22222222-2222-4222-8222-222222222222";
const PICTURE: &str = "33333333-3333-4333-8333-333333333333";
const GROUP: &str = "44444444-4444-4444-8444-444444444444";
const CATALOG: &str = "55555555-5555-4555-8555-555555555555";
const CATALOG_REF: &str = "66666666-6666-4666-8666-666666666666";
const ATTRIBUTE: &str = "77777777-7777-4777-8777-777777777777";

/// A source tree under the temporary directory, removed on drop.
struct Tree(PathBuf);

impl Tree {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "ibcmd-metadata-source-seam-{}",
            uuid::Uuid::new_v4().hyphenated()
        ));
        let tree = Self(root);
        tree.write(
            "Configuration.xml",
            &format!("<Configuration uuid=\"{CONFIGURATION}\"><Properties><Name>Конфигурация</Name></Properties></Configuration>"),
        );
        tree.write(
            "StyleItems/Акцент.xml",
            &format!("<StyleItem uuid=\"{STYLE_ITEM}\"><Properties><Name>Акцент</Name></Properties></StyleItem>"),
        );
        // A file in the style items' folder that declares something else.
        tree.write(
            "StyleItems/Чужой.xml",
            &format!("<CommonPicture uuid=\"{PICTURE}\"><Properties><Name>Чужой</Name></Properties></CommonPicture>"),
        );
        tree.write(
            "CommonPictures/Логотип.xml",
            &format!("<CommonPicture uuid=\"{PICTURE}\"><Properties><Name>Логотип</Name></Properties></CommonPicture>"),
        );
        tree.write(
            "CommandGroups/Печать.xml",
            &format!("<CommandGroup uuid=\"{GROUP}\"><Properties><Name>Печать</Name></Properties></CommandGroup>"),
        );
        tree.write(
            "Catalogs/Товары.xml",
            &format!("<Catalog uuid=\"{CATALOG}\"><Properties><Name>Товары</Name><GeneratedTypes><GeneratedType name=\"CatalogRef.Товары\"><TypeId>{CATALOG_REF}</TypeId></GeneratedType></GeneratedTypes></Properties><ChildObjects><Attribute uuid=\"{ATTRIBUTE}\"><Properties><Name>Цена</Name></Properties></Attribute></ChildObjects></Catalog>"),
        );
        tree
    }

    fn write(&self, relative: &str, body: &str) {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<MetaDataObject xmlns=\"http://v8.1c.ru/8.3/MDClasses\" version=\"2.20\">{body}</MetaDataObject>\n"
            ),
        )
        .unwrap();
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The messages of an error's chain, outermost first, and the kind of the
/// I/O error at its root, if one is.
fn anatomy(error: &anyhow::Error) -> (Vec<String>, Option<ErrorKind>) {
    (
        error.chain().map(ToString::to_string).collect(),
        error
            .root_cause()
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
    )
}

/// Asserts that the trait and the inherent method answered alike, and
/// returns the answer.
fn same(through_trait: Result<String>, inherent: Result<String>) -> Result<String> {
    match (&through_trait, &inherent) {
        (Ok(left), Ok(right)) => assert_eq!(left, right),
        (Err(left), Err(right)) => assert_eq!(anatomy(left), anatomy(right)),
        _ => panic!("the trait answered {through_trait:?} where the context answered {inherent:?}"),
    }
    through_trait
}

#[test]
fn the_context_answers_through_the_trait_as_through_its_own_methods() {
    let tree = Tree::new();
    let context = MetadataSourceContext::new(tree.0.clone());
    let source: &dyn MetadataSource = &context;

    // The root and the memo are the context's own fields.
    assert!(std::ptr::eq(
        source.source_root(),
        context.source_root.as_path()
    ));
    assert_eq!(source.source_root(), tree.0.as_path());
    assert!(std::ptr::eq(
        source.interface_memo(),
        &*context.interface_memo
    ));

    // A file: the same bytes, read for the caller; a missing one, the same
    // I/O error.
    let configuration = tree.path("Configuration.xml");
    let (Ok(SourceBytes::Owned(left)), Ok(SourceBytes::Owned(right))) = (
        source.read_source(&configuration),
        MetadataSourceContext::read_source(&context, &configuration),
    ) else {
        panic!("the configuration file reads as owned bytes through both");
    };
    assert_eq!(left, right);
    assert_eq!(left, fs::read(&configuration).unwrap());
    let missing = tree.path("Нет.xml");
    let (Err(left), Err(right)) = (
        source.read_source(&missing),
        MetadataSourceContext::read_source(&context, &missing),
    ) else {
        panic!("a missing file reads through neither");
    };
    assert_eq!(left.kind(), ErrorKind::NotFound);
    assert_eq!(
        (left.kind(), left.to_string()),
        (right.kind(), right.to_string())
    );

    // Memoised answers: the same values, and the same shared objects.
    assert_eq!(source.configuration_uuid().as_deref(), Some(CONFIGURATION));
    assert_eq!(
        source.configuration_uuid(),
        MetadataSourceContext::configuration_uuid(&context)
    );
    let object = source.configuration_object("Catalog.Товары").unwrap();
    assert_eq!(
        (object.class.as_str(), object.uuid.as_str()),
        ("Catalog", CATALOG)
    );
    assert!(Arc::ptr_eq(
        &object,
        &MetadataSourceContext::configuration_object(&context, "Catalog.Товары").unwrap()
    ));
    assert!(source.configuration_object("Catalog.Нет").is_none());
    assert!(MetadataSourceContext::configuration_object(&context, "Catalog.Нет").is_none());
    assert!(std::ptr::eq(
        source.dcs_style_items(),
        MetadataSourceContext::dcs_style_items(&context)
    ));
    assert_eq!(
        source.dcs_style_items().get(STYLE_ITEM).map(String::as_str),
        Some("Акцент")
    );
    assert_eq!(
        source.dcs_generated_type_id("CatalogRef.Товары").as_deref(),
        Some(CATALOG_REF)
    );
    assert_eq!(
        source.dcs_generated_type_id("CatalogRef.Товары"),
        MetadataSourceContext::dcs_generated_type_id(&context, "CatalogRef.Товары")
    );
    assert_eq!(source.dcs_generated_type_id("DefinedType.Деньги"), None);
    let refs = source.moxel_object_refs().unwrap();
    assert_eq!(
        refs,
        MetadataSourceContext::moxel_object_refs(&context).unwrap()
    );
    assert_eq!(
        refs.get(PICTURE).map(String::as_str),
        Some("CommonPicture.Логотип")
    );
    assert_eq!(
        refs.get(STYLE_ITEM).map(String::as_str),
        Some("StyleItem.Акцент")
    );

    // The resolvers: the same value or the same error, message by message.
    let style = |reference: &str| {
        same(
            source.resolve_style_item_uuid(reference),
            MetadataSourceContext::resolve_style_item_uuid(&context, reference),
        )
    };
    assert_eq!(style("StyleItem.Акцент").unwrap(), STYLE_ITEM);
    assert_eq!(
        anatomy(&style("Color.Акцент").unwrap_err()),
        (
            vec!["unsupported StyleItem reference: Color.Акцент".to_string()],
            None
        )
    );
    let (messages, kind) = anatomy(&style("StyleItem.Нет").unwrap_err());
    assert_eq!(
        messages[0],
        format!(
            "failed to read StyleItem XML {}",
            tree.path("StyleItems/Нет.xml").display()
        )
    );
    assert_eq!((messages.len(), kind), (2, Some(ErrorKind::NotFound)));
    assert_eq!(
        anatomy(&style("StyleItem.Чужой").unwrap_err()),
        (
            vec![format!(
                "expected StyleItem XML at {}, got CommonPicture",
                tree.path("StyleItems/Чужой.xml").display()
            )],
            None
        )
    );

    let picture = |reference: &str| {
        same(
            source.resolve_common_picture_uuid(reference),
            MetadataSourceContext::resolve_common_picture_uuid(&context, reference),
        )
    };
    assert_eq!(picture("CommonPicture.Логотип").unwrap(), PICTURE);
    assert_eq!(
        anatomy(&picture("Picture.Логотип").unwrap_err()).0,
        ["unsupported CommonCommand Picture reference: Picture.Логотип"]
    );
    assert_eq!(
        anatomy(&picture("CommonPicture.Нет").unwrap_err()).1,
        Some(ErrorKind::NotFound)
    );

    let group = |reference: &str| {
        same(
            source.resolve_command_group_uuid(reference),
            MetadataSourceContext::resolve_command_group_uuid(&context, reference),
        )
    };
    assert_eq!(group("CommandGroup.Печать").unwrap(), GROUP);
    assert_eq!(
        anatomy(&group("Group.Печать").unwrap_err()).0,
        ["unsupported CommandGroup reference: Group.Печать"]
    );
    assert_eq!(
        anatomy(&group("CommandGroup.Нет").unwrap_err()).0[0],
        format!(
            "failed to read CommandGroup XML {}",
            tree.path("CommandGroups/Нет.xml").display()
        )
    );

    let object = |reference: &str| {
        same(
            source.resolve_metadata_reference_uuid(reference),
            MetadataSourceContext::resolve_metadata_reference_uuid(&context, reference),
        )
    };
    assert_eq!(object("Catalog.Товары").unwrap(), CATALOG);
    assert_eq!(
        anatomy(&object("Нечто.Товары").unwrap_err()).0,
        ["unsupported metadata reference for source resolution: Нечто.Товары"]
    );
    assert_eq!(
        anatomy(&object("Catalog.Нет").unwrap_err()).0[0],
        format!(
            "failed to read Catalog XML {}",
            tree.path("Catalogs/Нет.xml").display()
        )
    );

    let type_id = |reference: &str| {
        same(
            source.resolve_metadata_type_id(reference),
            MetadataSourceContext::resolve_metadata_type_id(&context, reference),
        )
    };
    assert_eq!(type_id("cfg:CatalogRef.Товары").unwrap(), CATALOG_REF);
    assert_eq!(
        anatomy(&type_id("cfg:НечтоRef.Товары").unwrap_err()).0,
        ["unsupported metadata type reference for source resolution: cfg:НечтоRef.Товары"]
    );
    assert_eq!(
        anatomy(&type_id("cfg:CatalogRef.Нет").unwrap_err()).0[0],
        format!(
            "failed to read metadata XML {}",
            tree.path("Catalogs/Нет.xml").display()
        )
    );
}

/// A help reference memoises its answer, an error as its message; through
/// the trait, the first answer and the memoised one are the context's.
#[test]
fn a_help_reference_answers_and_memoises_through_the_trait_as_the_context_does() {
    let tree = Tree::new();
    let through_trait = MetadataSourceContext::new(tree.0.clone());
    let inherent = MetadataSourceContext::new(tree.0.clone());
    let source: &dyn MetadataSource = &through_trait;
    for _ in 0..2 {
        let (Ok(HelpReference::Resolved(left)), Ok(HelpReference::Resolved(right))) = (
            source.help_reference("Catalog.Товары"),
            inherent.help_reference("Catalog.Товары"),
        ) else {
            panic!("the catalog resolves through both");
        };
        assert_eq!((left.as_str(), right.as_str()), (CATALOG, CATALOG));
        assert!(matches!(
            (
                source.help_reference("Catalog.Нет"),
                inherent.help_reference("Catalog.Нет")
            ),
            (Ok(HelpReference::Absent), Ok(HelpReference::Absent))
        ));
        let (Err(left), Err(right)) = (
            source.help_reference("StyleItem.Чужой"),
            inherent.help_reference("StyleItem.Чужой"),
        ) else {
            panic!("a file that declares another class refuses through both");
        };
        assert_eq!(anatomy(&left), anatomy(&right));
        assert_eq!(
            anatomy(&left),
            (
                vec![format!(
                    "{} declares CommonPicture.Чужой",
                    tree.path("StyleItems/Чужой.xml").display()
                )],
                None
            )
        );
    }
}
