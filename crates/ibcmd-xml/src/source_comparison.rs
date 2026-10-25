//! Structural source comparison for metadata import. Scalar text stays exact.

use crate::{AttributeKind, XmlDocument, XmlElement, XmlNode, XmlReader};

fn attributes(element: &XmlElement) -> Vec<(String, &str)> {
    let mut values = element
        .attributes()
        .iter()
        .map(|a| {
            let key = match a.kind() {
                AttributeKind::Ordinary(name) => format!("a:{}", name.raw()),
                AttributeKind::Namespace(prefix) => {
                    format!("n:{}", prefix.as_deref().unwrap_or(""))
                }
            };
            (key, a.value())
        })
        .collect::<Vec<_>>();
    values.sort_unstable();
    values
}
fn preserve_space(element: &XmlElement, inherited: bool) -> bool {
    element
        .attributes()
        .iter()
        .find_map(|a| match a.kind() {
            AttributeKind::Ordinary(name) if name.raw() == "xml:space" => {
                Some(a.value() != "default")
            }
            _ => None,
        })
        .unwrap_or(inherited)
}
fn indentation(nodes: &[XmlNode]) -> bool {
    nodes.iter().any(|n| matches!(n, XmlNode::Element(_)))
        && nodes.iter().all(|n| match n {
            XmlNode::Text(t) => t
                .value()
                .bytes()
                .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n')),
            XmlNode::Element(_) | XmlNode::Comment(_) | XmlNode::ProcessingInstruction(_) => true,
            _ => false,
        })
}
fn nodes(nodes: &[XmlNode], structural: bool) -> Vec<&XmlNode> {
    nodes
        .iter()
        .filter(|n| !structural || !matches!(n, XmlNode::Text(_)))
        .collect()
}
/// Compare an element's data while allowing attribute order, indentation and
/// empty-element spelling. Namespace bindings and scalar whitespace are data.
pub fn source_elements_equal(left: &XmlElement, right: &XmlElement) -> bool {
    let mut pending = vec![(left, right, false)];
    while let Some((left, right, inherited)) = pending.pop() {
        if left.name() != right.name() || attributes(left) != attributes(right) {
            return false;
        }
        let preserve = preserve_space(left, inherited);
        let structural = !preserve && indentation(left.children()) && indentation(right.children());
        let a = nodes(left.children(), structural);
        let b = nodes(right.children(), structural);
        if a.len() != b.len() {
            return false;
        }
        for (a, b) in a.into_iter().zip(b) {
            match (a, b) {
                (XmlNode::Element(a), XmlNode::Element(b)) => pending.push((a, b, preserve)),
                (XmlNode::Text(a), XmlNode::Text(b)) if a.value() == b.value() => {}
                (XmlNode::CData(a), XmlNode::CData(b)) if a.value() == b.value() => {}
                (XmlNode::Comment(a), XmlNode::Comment(b)) if a.value() == b.value() => {}
                (XmlNode::ProcessingInstruction(a), XmlNode::ProcessingInstruction(b))
                    if a.value() == b.value() => {}
                _ => return false,
            }
        }
    }
    true
}
fn outside(nodes: &[XmlNode]) -> Vec<&XmlNode> {
    nodes
        .iter()
        .filter(|n| !matches!(n, XmlNode::Text(t) if t.value().trim().is_empty()))
        .collect()
}
pub(crate) fn documents_equal(left: &XmlDocument, right: &XmlDocument) -> bool {
    outside(left.before_root()) == outside(right.before_root())
        && outside(left.after_root()) == outside(right.after_root())
        && source_elements_equal(left.root(), right.root())
}
/// Formatting equality for an XML metadata source. Invalid XML never matches.
/// BOM and declaration spelling do not change its metadata; document-type and
/// processing-instruction changes are retained in the comparison.
pub fn equal_ignoring_source_formatting(left: &[u8], right: &[u8]) -> bool {
    match (XmlReader::from_slice(left), XmlReader::from_slice(right)) {
        (Ok(left), Ok(right)) => documents_equal(&left, &right),
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::equal_ignoring_source_formatting as same;
    #[test]
    fn formatting_is_not_scalar_or_namespace_data() {
        assert!(same(
            b"<r b='2' a='1'><x/><y> keep </y></r>",
            b"\xef\xbb\xbf<?xml version='1.0'?>\n<r a=\"1\" b=\"2\">\n  <x></x><y> keep </y>\n</r>"
        ));
        for (a, b) in [
            ("<r><x> keep </x></r>", "<r><x>keep</x></r>"),
            ("<r xmlns='old'/>", "<r xmlns='new'/>"),
            ("<r><x/><y/></r>", "<r><y/><x/></r>"),
            (
                "<r xml:space='preserve'> <x/></r>",
                "<r xml:space='preserve'><x/></r>",
            ),
            ("<r><x> </x></r>", "<r><x/></r>"),
        ] {
            assert!(!same(a.as_bytes(), b.as_bytes()), "{a:?} {b:?}");
        }
        assert!(!same(b"<r>", b"<r>"));
    }
}
