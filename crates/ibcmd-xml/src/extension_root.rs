//! Semantic admission of extension root edits; it grants no SQL permission.
use crate::source_comparison::{documents_equal, source_elements_equal};
use crate::{QName, XmlDocument, XmlElement, XmlNode, XmlReader};
use std::collections::BTreeSet;

#[derive(Debug, PartialEq, Eq)]
pub struct ExtensionRootEdit {
    pub version: Option<(String, String)>,
}
fn elements(e: &XmlElement) -> impl Iterator<Item = &XmlElement> {
    e.children().iter().filter_map(|n| {
        if let XmlNode::Element(e) = n {
            Some(e)
        } else {
            None
        }
    })
}
fn one<'a>(e: &'a XmlElement, name: &str) -> Result<&'a XmlElement, Vec<String>> {
    let found = elements(e)
        .filter(|e| e.name().raw() == name)
        .collect::<Vec<_>>();
    match found.as_slice() {
        [e] => Ok(e),
        _ => Err(vec![name.into()]),
    }
}
fn configuration(d: &XmlDocument) -> Result<&XmlElement, Vec<String>> {
    match d.root().name().raw() {
        "Configuration" => Ok(d.root()),
        "MetaDataObject" => one(d.root(), "Configuration"),
        _ => Err(vec!["Configuration envelope".into()]),
    }
}
fn version(p: &XmlElement) -> Result<Option<String>, Vec<String>> {
    let found = elements(p)
        .filter(|e| e.name().raw() == "Version")
        .collect::<Vec<_>>();
    let e = match found.as_slice() {
        [] => return Ok(None),
        [e] => e,
        _ => return Err(vec!["Properties/Version".into()]),
    };
    if !e.attributes().is_empty() || e.children().iter().any(|n| !matches!(n, XmlNode::Text(_))) {
        return Err(vec!["Properties/Version".into()]);
    }
    Ok(Some(
        e.children()
            .iter()
            .filter_map(|n| {
                if let XmlNode::Text(t) = n {
                    Some(t.value())
                } else {
                    None
                }
            })
            .collect(),
    ))
}
fn valid_version(value: &str) -> bool {
    value.split('.').count() >= 2
        && value
            .split('.')
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}
fn placeholder(name: &str) -> XmlNode {
    XmlNode::Element(XmlElement::new(
        QName::new(name).expect("static policy element name"),
    ))
}
fn mask_properties(properties: &XmlElement) -> XmlElement {
    let children = properties
        .children()
        .iter()
        .map(|node| match node {
            XmlNode::Element(element) if element.name().raw() == "Version" => {
                placeholder("Version")
            }
            _ => node.clone(),
        })
        .collect();
    properties.with_children(children)
}
fn mask_collections(collections: &XmlElement, allowed: &impl Fn(&str) -> bool) -> XmlElement {
    let mut children = collections
        .children()
        .iter()
        .filter(|node| {
            !matches!(node, XmlNode::Element(element)
            if allowed(element.name().raw()) && element.attributes().is_empty()
                && element.children().iter().all(|child| matches!(child, XmlNode::Text(_))))
        })
        .cloned()
        .collect::<Vec<_>>();
    // Retain an element-only container even when every allowed list was
    // removed, so indentation is never mistaken for scalar text.
    children.push(placeholder("ComparedCollections"));
    collections.with_children(children)
}
fn mask(configuration: &XmlElement, allowed: &impl Fn(&str) -> bool) -> XmlElement {
    let children = configuration
        .children()
        .iter()
        .map(|node| match node {
            XmlNode::Element(element) if element.name().raw() == "Properties" => {
                XmlNode::Element(mask_properties(element))
            }
            XmlNode::Element(element) if element.name().raw() == "ChildObjects" => {
                XmlNode::Element(mask_collections(element, allowed))
            }
            _ => node.clone(),
        })
        .collect();
    configuration.with_children(children)
}
fn masked_document(d: &XmlDocument, allowed: &impl Fn(&str) -> bool) -> XmlDocument {
    if d.root().name().raw() == "Configuration" {
        return d.with_root(mask(d.root(), allowed));
    }
    d.with_root(
        d.root().with_children(
            d.root()
                .children()
                .iter()
                .map(|n| match n {
                    XmlNode::Element(e) if e.name().raw() == "Configuration" => {
                        XmlNode::Element(mask(e, allowed))
                    }
                    _ => n.clone(),
                })
                .collect(),
        ),
    )
}
/// Compare complete roots after masking only Version and caller-admitted
/// child collections. Errors name real changed properties or malformed paths.
pub fn extension_root_edit(
    active: &[u8],
    edited: &[u8],
    allowed: impl Fn(&str) -> bool,
) -> Result<ExtensionRootEdit, Vec<String>> {
    let parse =
        |s| XmlReader::from_slice(s).map_err(|e| vec![format!("invalid Configuration.xml: {e}")]);
    let (a, b) = (parse(active)?, parse(edited)?);
    let (x, y) = (configuration(&a)?, configuration(&b)?);
    let (xp, yp) = (one(x, "Properties")?, one(y, "Properties")?);
    one(x, "ChildObjects")?;
    one(y, "ChildObjects")?;
    let (old, new) = (version(xp)?, version(yp)?);
    let version = if old == new {
        None
    } else {
        let (Some(old), Some(new)) = (old, new) else {
            return Err(vec!["Properties/Version".into()]);
        };
        if !valid_version(&old)
            || !valid_version(&new)
            || old.split('.').count() != new.split('.').count()
        {
            return Err(vec!["Properties/Version".into()]);
        }
        Some((old, new))
    };
    if documents_equal(
        &masked_document(&a, &allowed),
        &masked_document(&b, &allowed),
    ) {
        return Ok(ExtensionRootEdit { version });
    }
    let mut changed = Vec::new();
    let names = elements(xp)
        .chain(elements(yp))
        .map(|e| e.name().raw())
        .filter(|n| *n != "Version")
        .collect::<BTreeSet<_>>();
    for name in names {
        let a = elements(xp)
            .filter(|e| e.name().raw() == name)
            .collect::<Vec<_>>();
        let b = elements(yp)
            .filter(|e| e.name().raw() == name)
            .collect::<Vec<_>>();
        if a.len() != b.len()
            || !a
                .into_iter()
                .zip(b)
                .all(|(a, b)| source_elements_equal(a, b))
        {
            changed.push(format!("Properties/{name}"));
        }
    }
    if changed.is_empty() {
        changed.push("Configuration identity, envelope or ChildObjects".into());
    }
    Err(changed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formatted_version_and_child_collection_compose_without_masking_properties() {
        let a = "<Configuration uuid='root'><Properties><Comment> keep </Comment><Version>1.2.0.4</Version></Properties><ChildObjects/></Configuration>";
        let b = "<Configuration uuid=\"root\">\n  <Properties><Comment> keep </Comment><Version>1.2.0.5</Version></Properties>\n  <ChildObjects><CommonModule>Added</CommonModule></ChildObjects>\n</Configuration>";
        assert_eq!(
            extension_root_edit(a.as_bytes(), b.as_bytes(), |n| n == "CommonModule")
                .unwrap()
                .version,
            Some(("1.2.0.4".into(), "1.2.0.5".into()))
        );
        assert_eq!(
            extension_root_edit(a.as_bytes(), b.replace(" keep ", "keep").as_bytes(), |n| n
                == "CommonModule")
            .unwrap_err(),
            vec!["Properties/Comment"]
        );
        assert!(extension_root_edit(a.as_bytes(), b.as_bytes(), |_| false).is_err());
        assert!(
            extension_root_edit(
                a.as_bytes(),
                b.replace("uuid=\"root\"", "uuid=\"changed\"").as_bytes(),
                |_| true
            )
            .is_err()
        );
        assert!(
            extension_root_edit(
                a.as_bytes(),
                b.replace("1.2.0.5", " 1.2.0.5").as_bytes(),
                |_| true
            )
            .is_err()
        );
    }
}
