//! What the form reader and the 8.5 down-conversion read of an adopted
//! form's body: its event block, every handler of every event with its
//! call type code, and the slot of its base form record (from
//! `src/mssql_dump/form_extension.rs`, which keeps the adoption model).

use ibcmd_1c_text::{parse_1c_quoted_string_with_len, split_1c_braced_fields};

use crate::ParsedFormBodyBlob;

/// One handler of one event, as stored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormEventBinding {
    pub event: String,
    pub handler: String,
    pub code: u8,
}

pub const CALL_TYPES: [&str; 3] = ["Before", "After", "Override"];

/// A complete quoted handler token. Bytes after its closing quote are
/// unsupported data, rather than part of the event record we can discard.
fn parse_event_handler(field: &str) -> Option<String> {
    let token = field.trim();
    let (handler, consumed) = parse_1c_quoted_string_with_len(token)?;
    (consumed == token.len()).then_some(handler)
}

/// The bindings of the event block `field`, in stored order: an event's first
/// handler (when it has one), then its further handlers.
pub fn parse_form_event_block(field: &str) -> Option<Vec<FormEventBinding>> {
    let fields = split_1c_braced_fields(field.trim(), 0)?;
    let count: usize = fields.first()?.trim().parse().ok()?;
    if count == 0 || count > fields.len() || fields.len() < 3 + 5 * count {
        return None;
    }
    let mut first = Vec::with_capacity(count);
    for index in 0..count {
        let event = fields.get(1 + 2 * index)?.trim();
        let handler = parse_event_handler(fields.get(2 + 2 * index)?)?;
        first.push((event, handler));
    }
    let mut at = 1 + 2 * count;
    if fields.get(at)?.trim() != "1" || fields.get(at + 1)?.trim() != "0" {
        return None;
    }
    at += 2;
    let mut out = Vec::new();
    for (event, handler) in first {
        if fields.get(at)?.trim() != event {
            return None;
        }
        let code: u8 = fields.get(at + 1)?.trim().parse().ok()?;
        let handlers: usize = fields.get(at + 2)?.trim().parse().ok()?;
        if handlers == 0 || usize::from(code) >= CALL_TYPES.len() {
            return None;
        }
        at += 3;
        if !handler.is_empty() {
            out.push(FormEventBinding {
                event: event.to_owned(),
                handler,
                code,
            });
        }
        for _ in 1..handlers {
            let extra = parse_event_handler(fields.get(at)?)?;
            let code: u8 = fields.get(at + 1)?.trim().parse().ok()?;
            if usize::from(code) >= CALL_TYPES.len() {
                return None;
            }
            at += 2;
            out.push(FormEventBinding {
                event: event.to_owned(),
                handler: extra,
                code,
            });
        }
    }
    (at == fields.len()).then_some(out)
}

/// The trailing section of an adopted form that holds its base form record.
pub fn adopted_base_record_slot(body: &ParsedFormBodyBlob) -> Option<usize> {
    let flag = body.trailing.get(5)?.trim();
    let base = body.trailing.get(6)?.trim();
    if flag != "1" || !base.starts_with('{') {
        return None;
    }
    Some(6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_beyond_the_fields_is_no_event_block() {
        assert!(parse_form_event_block("{18446744073709551615,e,\"h\",1,0,e,0,1}").is_none());
    }

    #[test]
    fn first_handler_token_must_be_consumed_completely() {
        assert!(parse_form_event_block(r#"{1,e,"Handler"UNMODELED,1,0,e,0,1}"#).is_none());
    }

    #[test]
    fn additional_handler_token_must_be_consumed_completely() {
        assert!(
            parse_form_event_block(r#"{1,e,"",1,0,e,0,2,"AfterHandler"UNMODELED,1}"#).is_none()
        );
    }

    #[test]
    fn complete_handler_tokens_keep_whitespace_and_doubled_quotes() {
        let bindings =
            parse_form_event_block(r#"{1,e, "First""Handler" ,1,0,e,0,2, "After""Handler" ,1}"#)
                .unwrap();
        assert_eq!(bindings[0].handler, "First\"Handler");
        assert_eq!(bindings[1].handler, "After\"Handler");
    }

    #[test]
    fn reads_every_handler_of_an_interceptor_block() {
        let block = "{3,3ccc650e-f631-4cae-8e33-3eaac610b5f9,\"Перед\",52dbb775-1631-4fd5-8c55-1615b5881dac,\
                     \"Вместо\",9f2e5ddb-3492-4f5d-8f0d-416b8d1d5c5b,\"\",1,0,\
                     3ccc650e-f631-4cae-8e33-3eaac610b5f9,0,1,52dbb775-1631-4fd5-8c55-1615b5881dac,2,1,\
                     9f2e5ddb-3492-4f5d-8f0d-416b8d1d5c5b,0,2,\"После\",1}";
        let bindings = parse_form_event_block(block).unwrap();
        let spelled = bindings
            .iter()
            .map(|b| (b.handler.as_str(), CALL_TYPES[usize::from(b.code)]))
            .collect::<Vec<_>>();
        assert_eq!(
            spelled,
            vec![
                ("Перед", "Before"),
                ("Вместо", "Override"),
                ("После", "After")
            ]
        );
        assert_eq!(bindings[2].event, "9f2e5ddb-3492-4f5d-8f0d-416b8d1d5c5b");
    }

    #[test]
    fn a_configuration_block_reads_one_handler_per_event() {
        let block = "{1,9f2e5ddb-3492-4f5d-8f0d-416b8d1d5c5b,\"ПриСозданииНаСервере\",1,0,\
                     9f2e5ddb-3492-4f5d-8f0d-416b8d1d5c5b,0,1}";
        assert_eq!(parse_form_event_block(block).unwrap().len(), 1);
        assert!(parse_form_event_block("{1,x,\"h\",1,0,y,0,1}").is_none());
    }
}
