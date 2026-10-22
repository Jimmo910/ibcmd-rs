//! What the 8.5 platform writes into a data-composition schema template
//! beyond what its 8.3.27 reading spells: a picture value of the settings
//! (`<dcscor:value xsi:type="v8ui:Picture" ref="v8ui:<name>"/>` in 2.20) is
//! written as an `xr:Picture` element naming the picture by its kind
//! (`CommonPicture.<name>`, `StdPicture.<name>`) with `LoadTransparent`
//! `true`, under a generated prefix numbered by the value element's depth.
//!
//! Evidence: the БСП 3.1.11 (8.3.27 XML) loaded by ibcmd 8.5.1.1529 into a
//! file infobase and exported by it: all 120 picture values of its templates
//! (114 common pictures, 5 standard, under `d8p1` at depth 8 and one
//! `d9p1` at depth 9) take this shape and none keeps the 2.20 one; the
//! templates of an 8.5.1.1529 export of a БСП-based `.cf` (`obf_1cv8` corpus,
//! the same five templates) do the same. The forms of the same exports keep
//! no picture value this way, so only templates are rewritten.

use std::collections::BTreeMap;

use anyhow::{Result, bail};

use super::form::xml_2_21_writer::XmlEdits;

const VALUE_TAG: &str = "dcscor:value";
const PICTURE_TYPE: &str = "xsi:type=\"v8ui:Picture\"";
const REF_ATTRIBUTE: &str = " ref=\"v8ui:";
const READABLE_NS: &str = "http://v8.1c.ru/8.3/xcf/readable";
const COMMON_PICTURE: &str = "CommonPicture.";
const STD_PICTURE: &str = "StdPicture.";

/// `xml` (a written schema template) with every picture value spelled the
/// 8.5 way.
pub(super) fn with_8_5_picture_values(
    xml: &str,
    object_refs: &BTreeMap<String, String>,
) -> Result<String> {
    if !xml.contains(PICTURE_TYPE) {
        return Ok(xml.to_owned());
    }
    let mut edits = XmlEdits::new(xml)?;
    let mut changed = false;
    for element in 0..edits.elements.len() {
        let item = &edits.elements[element];
        if item.tag != VALUE_TAG || !item.self_closing {
            continue;
        }
        let open = &xml[item.open_start..];
        let open = &open[..open.find('>').unwrap_or(open.len())];
        if !open.contains(PICTURE_TYPE) {
            continue;
        }
        let Some(at) = open.find(REF_ATTRIBUTE) else {
            bail!("a picture value without a picture reference: {open}");
        };
        let name = &open[at + REF_ATTRIBUTE.len()..];
        let name = &name[..name.find('"').unwrap_or(name.len())];
        let reference = picture_reference(name, object_refs);
        let mut depth = 1;
        let mut parent = item.parent;
        while let Some(index) = parent {
            depth += 1;
            parent = edits.elements[index].parent;
        }
        let prefix = format!("d{depth}p1");
        edits.replace(
            element,
            &format!(
                "<{VALUE_TAG} xmlns:{prefix}=\"{READABLE_NS}\" xsi:type=\"{prefix}:Picture\">\r\n\
                 \t<{prefix}:Ref>{reference}</{prefix}:Ref>\r\n\
                 \t<{prefix}:LoadTransparent>true</{prefix}:LoadTransparent>\r\n\
                 </{VALUE_TAG}>\r\n"
            ),
        );
        changed = true;
    }
    if !changed {
        return Ok(xml.to_owned());
    }
    edits.finish()
}

/// `CommonPicture.<name>` for a picture of the configuration, else
/// `StdPicture.<name>`: a settings picture is stored either by the uuid the
/// 2.20 writer resolved to a common or a standard picture, or by the name
/// of a platform picture (`GrayedAll`, `EventLog`, `Information` in the БСП
/// templates, which 8.5.1.1529 writes as `StdPicture.*`; `GrayedAll` is in
/// no measured uuid table).
fn picture_reference(name: &str, object_refs: &BTreeMap<String, String>) -> String {
    let common = format!("{COMMON_PICTURE}{name}");
    if object_refs.values().any(|reference| *reference == common) {
        common
    } else {
        format!("{STD_PICTURE}{name}")
    }
}

/// The inverse, for a compiled source in dialect 2.21: every picture value
/// spelled the 8.5 way back in the spelling the platform stores (`v8ui:`
/// and the bare name, for a common and a standard picture alike; the БСП
/// templates its 8.5.1.1529 import of the 8.3.27 XML stored hold 89 such
/// values in one template and no other spelling). `None` when nothing is
/// spelled that way. A value of another shape (an inline picture, `Abs`) is
/// left as written.
pub(crate) fn with_stored_picture_values(xml: &str) -> Result<Option<String>> {
    if !xml.contains(READABLE_NS) {
        return Ok(None);
    }
    let mut edits = XmlEdits::new(xml)?;
    let mut changed = false;
    for element in 0..edits.elements.len() {
        let item = &edits.elements[element];
        if item.tag != VALUE_TAG || item.self_closing {
            continue;
        }
        let open = &xml[item.open_start..];
        let open = &open[..open.find('>').unwrap_or(open.len())];
        let Some(prefix) = readable_picture_prefix(open) else {
            continue;
        };
        let lines: Vec<&str> = edits
            .inner_lines(element)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let reference = match lines.as_slice() {
            [reference, transparent]
                if *transparent
                    == format!("<{prefix}:LoadTransparent>true</{prefix}:LoadTransparent>") =>
            {
                reference
                    .strip_prefix(&format!("<{prefix}:Ref>"))
                    .and_then(|rest| rest.strip_suffix(&format!("</{prefix}:Ref>")))
            }
            _ => None,
        };
        let Some(name) = reference.and_then(|reference| {
            reference
                .strip_prefix(COMMON_PICTURE)
                .or_else(|| reference.strip_prefix(STD_PICTURE))
        }) else {
            continue;
        };
        edits.replace(
            element,
            &format!("<{VALUE_TAG} {PICTURE_TYPE}{REF_ATTRIBUTE}{name}\"/>\r\n"),
        );
        changed = true;
    }
    if !changed {
        return Ok(None);
    }
    edits.finish().map(Some)
}

/// The prefix of `open` (a value's start tag) when it declares the readable
/// namespace under a generated prefix and types the value as its `Picture`.
fn readable_picture_prefix(open: &str) -> Option<&str> {
    let declaration = format!("=\"{READABLE_NS}\"");
    let at = open.find(&declaration)?;
    let prefix = open[..at].rsplit_once("xmlns:")?.1;
    open.contains(&format!("xsi:type=\"{prefix}:Picture\""))
        .then_some(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_8_5_spelling_is_stored_the_platforms_way() {
        let refs = BTreeMap::from([(
            "00000000-0000-0000-0000-000000000001".to_owned(),
            "CommonPicture.Своя".to_owned(),
        )]);
        let stored = "<a>\r\n\
                      \t<b>\r\n\
                      \t\t<dcscor:value xsi:type=\"v8ui:Picture\" ref=\"v8ui:Своя\"/>\r\n\
                      \t</b>\r\n\
                      \t<dcscor:value xsi:type=\"v8ui:Picture\" ref=\"v8ui:AppearanceCheckBox\"/>\r\n\
                      </a>";
        let written = with_8_5_picture_values(stored, &refs).unwrap();
        assert_eq!(
            with_stored_picture_values(&written).unwrap().as_deref(),
            Some(stored)
        );
        assert_eq!(with_stored_picture_values(stored).unwrap(), None);
    }

    #[test]
    fn an_inline_picture_is_left_as_written() {
        let xml = "<a>\r\n\
                   \t<dcscor:value xmlns:d2p1=\"http://v8.1c.ru/8.3/xcf/readable\" xsi:type=\"d2p1:Picture\">\r\n\
                   \t\t<d2p1:Abs>value.svg</d2p1:Abs>\r\n\
                   \t\t<d2p1:LoadTransparent>true</d2p1:LoadTransparent>\r\n\
                   \t</dcscor:value>\r\n\
                   </a>";
        assert_eq!(with_stored_picture_values(xml).unwrap(), None);
    }

    #[test]
    fn picture_values_name_their_kind_under_a_depth_prefix() {
        let xml = "<a>\r\n\
                   \t<b>\r\n\
                   \t\t<dcscor:value xsi:type=\"v8ui:Picture\" ref=\"v8ui:Своя\"/>\r\n\
                   \t</b>\r\n\
                   \t<dcscor:value xsi:type=\"v8ui:Picture\" ref=\"v8ui:AppearanceCheckBox\"/>\r\n\
                   \t<dcscor:value xsi:type=\"xs:string\">x</dcscor:value>\r\n\
                   </a>";
        let refs = BTreeMap::from([(
            "00000000-0000-0000-0000-000000000001".to_owned(),
            "CommonPicture.Своя".to_owned(),
        )]);
        assert_eq!(
            with_8_5_picture_values(xml, &refs).unwrap(),
            "<a>\r\n\
             \t<b>\r\n\
             \t\t<dcscor:value xmlns:d3p1=\"http://v8.1c.ru/8.3/xcf/readable\" xsi:type=\"d3p1:Picture\">\r\n\
             \t\t\t<d3p1:Ref>CommonPicture.Своя</d3p1:Ref>\r\n\
             \t\t\t<d3p1:LoadTransparent>true</d3p1:LoadTransparent>\r\n\
             \t\t</dcscor:value>\r\n\
             \t</b>\r\n\
             \t<dcscor:value xmlns:d2p1=\"http://v8.1c.ru/8.3/xcf/readable\" xsi:type=\"d2p1:Picture\">\r\n\
             \t\t<d2p1:Ref>StdPicture.AppearanceCheckBox</d2p1:Ref>\r\n\
             \t\t<d2p1:LoadTransparent>true</d2p1:LoadTransparent>\r\n\
             \t</dcscor:value>\r\n\
             \t<dcscor:value xsi:type=\"xs:string\">x</dcscor:value>\r\n\
             </a>"
        );
    }

    #[test]
    fn a_value_with_no_reference_is_refused() {
        let xml = "<a>
	<dcscor:value xsi:type=\"v8ui:Picture\"/>
</a>";
        assert!(with_8_5_picture_values(xml, &BTreeMap::new()).is_err());
    }

    #[test]
    fn a_template_without_pictures_is_left_alone() {
        let xml = "<a>\r\n\t<b/>\r\n</a>";
        assert_eq!(with_8_5_picture_values(xml, &BTreeMap::new()).unwrap(), xml);
    }
}
