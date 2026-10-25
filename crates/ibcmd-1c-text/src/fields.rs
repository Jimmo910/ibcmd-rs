//! Scalar fields of 1C braced values: UUIDs, `0`/`1` flags, unsigned
//! numbers, `YYYYMMDDhhmmss` dates, and the payload of a `{#base64:...}`
//! value; and the scan of a text for the UUIDs it names.

use std::collections::BTreeSet;

pub fn parse_uuid_field(value: &str) -> Option<String> {
    let value = value.trim();
    if is_uuid_text(value) {
        Some(value.to_string())
    } else {
        None
    }
}

pub fn parse_non_zero_uuid(value: &str) -> Option<String> {
    let uuid = parse_uuid_field(value)?;
    if uuid == "00000000-0000-0000-0000-000000000000" {
        None
    } else {
        Some(uuid)
    }
}

pub fn is_uuid_text(value: &str) -> bool {
    value.len() == 36 && value.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
}

pub fn parse_1c_bool_field(value: Option<&str>) -> Option<bool> {
    parse_1c_bool_flag(value?.trim())
}

// An absent legacy field can have a documented default. A present value
// outside the Boolean domain is a conversion refusal, never that default.
pub fn parse_1c_bool_field_or_default(value: Option<&str>, default: bool) -> Option<bool> {
    value.map_or(Some(default), |value| parse_1c_bool_field(Some(value)))
}

pub fn parse_1c_u32_field(value: Option<&str>) -> Option<u32> {
    value?.trim().parse().ok()
}

pub fn parse_1c_bool_flag(value: &str) -> Option<bool> {
    match value {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

pub fn extract_base64_payload(text: &str) -> Option<&str> {
    let prefix = "{#base64:";
    let start = text.find(prefix)? + prefix.len();
    let end = text[start..].find('}')? + start;
    Some(&text[start..end])
}

pub fn uuid_like_values(text: &str) -> BTreeSet<String> {
    uuid_like_values_in_text_order(text).into_iter().collect()
}

pub fn uuid_like_values_in_text_order(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut values = Vec::new();
    let mut seen = BTreeSet::new();
    if bytes.len() < 36 {
        return values;
    }
    for start in 0..=bytes.len() - 36 {
        let value = &bytes[start..start + 36];
        if is_uuid_like_ascii(value) {
            let value = String::from_utf8_lossy(value).to_ascii_lowercase();
            if seen.insert(value.clone()) {
                values.push(value);
            }
        }
    }
    values
}

pub fn is_uuid_like_ascii(value: &[u8]) -> bool {
    if value.len() != 36 {
        return false;
    }
    for (index, byte) in value.iter().copied().enumerate() {
        if matches!(index, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

pub fn decode_base64_mime(input: &str) -> Option<Vec<u8>> {
    let values = input
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if values.len() % 4 != 0 {
        return None;
    }

    let mut output = Vec::with_capacity(values.len() / 4 * 3);
    for chunk in values.chunks(4) {
        let mut decoded = [0u8; 4];
        let mut padding = 0usize;
        for (index, byte) in chunk.iter().copied().enumerate() {
            if byte == b'=' {
                padding += 1;
                decoded[index] = 0;
                continue;
            }
            if padding > 0 {
                return None;
            }
            decoded[index] = base64_value(byte)?;
        }
        if padding > 2 {
            return None;
        }
        output.push((decoded[0] << 2) | (decoded[1] >> 4));
        if padding < 2 {
            output.push((decoded[1] << 4) | (decoded[2] >> 2));
        }
        if padding < 1 {
            output.push((decoded[2] << 6) | decoded[3]);
        }
    }

    Some(output)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

pub fn format_1c_date_time(value: &str) -> Option<String> {
    if value.len() != 14 || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some(format!(
        "{}-{}-{}T{}:{}:{}",
        &value[0..4],
        &value[4..6],
        &value[6..8],
        &value[8..10],
        &value[10..12],
        &value[12..14]
    ))
}
