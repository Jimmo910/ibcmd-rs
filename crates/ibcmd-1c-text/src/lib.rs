//! Primitives of the text 1C stores its records in: braced values
//! (`{field,field,...}`), the string literals inside them, a few scalar
//! fields, and the XML escaping the writers apply to what they read.
//!
//! Everything here moved unchanged from the root crate's `mssql_dump`
//! (`mod.rs`, and `extract_base64_payload` from `source_assets.rs`), which
//! still reaches every item by its old name through imports. The root's
//! form-body, reference and metadata readers register their texts in this
//! crate's per-thread jump table ([`register_brace_jumps`]); the splitters
//! here read the same table, so it has one definition, in this crate.
//!
//! The code came from the physical-adapter guard's slice, so the guard scans
//! every file of this crate (`tools/validate-physical-adapter-policy.ps1`,
//! `$GuardedCrateFiles`) and its literals stay in the guard's baseline.

mod braced;
mod fields;
mod strings;
mod xml;

pub use braced::{
    BRACE_JUMPS_MIN_TEXT, BraceJumpsGuard, expect_comma_at, for_each_1c_braced_field,
    register_brace_jumps, registered_header_marker, scan_1c_braced_value, skip_ascii_ws_at,
    split_1c_braced_fields, split_1c_braced_fields_bounded, split_1c_braced_sequence,
};
pub use fields::{
    extract_base64_payload, is_uuid_text, parse_1c_bool_field, parse_1c_bool_field_or_default,
    parse_1c_bool_flag, parse_1c_u32_field, parse_non_zero_uuid, parse_uuid_field,
};
pub use strings::{
    parse_1c_quoted_string, parse_1c_quoted_string_with_len, parse_1c_string, parse_1c_synonyms,
};
pub use xml::{escape_xml_character_data, escape_xml_element_text, escape_xml_text, xml_bool};
