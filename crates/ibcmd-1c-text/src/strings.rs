//! 1C string literals: `"..."`, where `""` is a quote and `"\xxxx` escapes
//! one UTF-16 code unit.

/// One UTF-16 code unit escaped inside a 1C string literal.
///
/// The literal escapes a code unit by interrupting itself: a quote, a
/// backslash, and exactly four lowercase hexadecimal digits, after which the
/// literal continues where it left off. `""` -- quote followed by quote -- is
/// the other, already-known interruption, the escaped quote itself.
///
/// Evidence, byte for byte. ERP УХ 3.2.12.6
/// `DataProcessors/ИнтеграцияС1СДокументооборот/Forms/БизнесПроцессРассмотрение`
/// spells the English title of attribute `ВариантУстановкиСрокаОбработкиРезультатов`
/// as `{"en","Specification option"\db40"\dc32"\db40"\dc21"…"\db40"\dc30 of
/// result processing deadline"}`, and the platform writes
/// `Specification option` followed by the twelve characters `U+E0032`,
/// `U+E0021`, `U+E0028`, `U+E0027`, `U+E0028`, `U+E0020`, `U+E0022`, `U+E0025`,
/// `U+E0023`, `U+E0024`, `U+E0028`, `U+E0033`, `U+E0030` and ` of result
/// processing deadline`. Each of those is a surrogate pair -- `db40` then the
/// matching low unit -- so the escapes come two at a time and combine into one
/// scalar. Every backslash of that whole form body, without exception,
/// conforms to the shape: 39 `db40` high units and their 39 low partners, and
/// no other backslash anywhere in it.
///
/// A high surrogate that is not immediately followed by its low partner, or a
/// low one with no partner, is a refusal rather than a replacement character:
/// an unpaired unit is not a scalar this reader is entitled to invent.
fn parse_1c_string_escape_unit(input: &str) -> Option<u16> {
    let rest = input.strip_prefix('\\')?;
    let hex = rest.get(..4)?;
    if !hex
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    u16::from_str_radix(hex, 16).ok()
}

pub fn parse_1c_quoted_string_with_len(input: &str) -> Option<(String, usize)> {
    if !input.starts_with('"') {
        return None;
    }
    let mut output = String::new();
    let mut pending_high: Option<u16> = None;
    let mut index = 1usize;
    while index < input.len() {
        let ch = input[index..].chars().next()?;
        if ch != '"' {
            if pending_high.is_some() {
                return None;
            }
            output.push(ch);
            index += ch.len_utf8();
            continue;
        }
        let rest = &input[index + 1..];
        if rest.starts_with('"') {
            if pending_high.is_some() {
                return None;
            }
            output.push('"');
            index += 2;
            continue;
        }
        if let Some(unit) = parse_1c_string_escape_unit(rest) {
            match unit {
                0xD800..=0xDBFF => {
                    if pending_high.is_some() {
                        return None;
                    }
                    pending_high = Some(unit);
                }
                0xDC00..=0xDFFF => {
                    let high = u32::from(pending_high.take()?);
                    let scalar = 0x10000 + ((high - 0xD800) << 10) + (u32::from(unit) - 0xDC00);
                    output.push(char::from_u32(scalar)?);
                }
                _ => {
                    if pending_high.is_some() {
                        return None;
                    }
                    output.push(char::from_u32(u32::from(unit))?);
                }
            }
            index += 6;
            continue;
        }
        if pending_high.is_some() {
            return None;
        }
        return Some((output, index + 1));
    }
    None
}

pub fn parse_1c_quoted_string(input: &str) -> Option<String> {
    parse_1c_quoted_string_with_len(input).map(|(value, _)| value)
}

pub fn parse_1c_synonyms(input: &str) -> Vec<(String, String)> {
    let mut values = Vec::new();
    let mut offset = 0;
    while let Some(relative) = input[offset..].find('"') {
        offset += relative;
        let Some((value, consumed)) = parse_1c_quoted_string_with_len(&input[offset..]) else {
            break;
        };
        values.push(value);
        offset += consumed;
    }

    values
        .chunks(2)
        .filter_map(|chunk| match chunk {
            [lang, content] => Some((lang.clone(), content.clone())),
            _ => None,
        })
        .collect()
}

pub fn parse_1c_string(text: &str) -> Option<String> {
    let text = text.trim();
    let inner = text.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\"\"", "\""))
}
