//! Index objects: B-trees of fixed-length keys.
//!
//! Page 0 of an index object holds the head of the free page chain and then
//! one `u32` per index: the page with that index's descriptor (`u32` root
//! page, `u16` key length, `u16` 0x0080). Every tree page starts with `u16`
//! flags (1 root, 2 leaf), `u16` entry count, `u32` previous and `u32` next
//! page on the same level (`0xFFFFFFFF` at the ends).
//!
//! A branch entry is the key of the last entry under the child, then that
//! entry's record number and the child page, both big-endian.
//!
//! A leaf adds `u16` free bytes, `u32` record mask, `u16` left and right
//! masks, `u16` record, left and right bit widths and `u16` bytes per entry.
//! Entries grow from the header, key bytes from the end of the page. An entry
//! packs the record number, `left` (bytes shared with the previous key) and
//! `right` (trailing zero bytes); the page stores only the middle of the key.
//!
//! Keys of the CONFIG-family tables (`FILENAME` NVC(128) case-insensitive,
//! `PARTNO` N(10)) are 392 bytes: an ICU-style sort key of the file name under
//! the infobase collation `en_US_POSIX` at secondary strength, then the packed
//! decimal of `PARTNO`, zero padded. See [`config_key`] for the measured
//! encoding and its limits.

use crate::error::{Error, Result, corrupt};

pub(crate) const CONFIG_KEY_LEN: usize = 392;
const LEAF_HEADER: usize = 30;
const BRANCH_HEADER: usize = 12;
const NONE: u32 = 0xFFFF_FFFF;
/// Longest file name whose secondary weight byte is evidenced (`0x04 + n`
/// measured for n ≤ 43 on native 8.5.1.1150 indexes).
pub const MAX_KEY_NAME: usize = 43;

const FLAG_ROOT: u16 = 1;
const FLAG_LEAF: u16 = 2;

/// Leaf entry layout of the CONFIG-family indexes as the platform writes it.
const REC_BITS: u16 = 32;
const LEFT_BITS: u16 = 16;
const RIGHT_BITS: u16 = 16;
const ENTRY_BYTES: u16 = 8;

/// Primary weight of one character under `en_US_POSIX`, or `None` outside the
/// evidenced range. The POSIX tailoring orders printable ASCII by code point;
/// every weight starts with 0x28, characters 0x21–0x5A take one more byte
/// (`c - 0x1C`), characters 0x5B–0x7E two (`0x3F`, `c - 0x57`).
fn primary(c: char) -> Option<&'static [u8]> {
    static ONE: [[u8; 2]; 0x5B - 0x21] = {
        let mut t = [[0u8; 2]; 0x5B - 0x21];
        let mut i = 0;
        while i < t.len() {
            t[i] = [0x28, (i + 0x21 - 0x1C) as u8];
            i += 1;
        }
        t
    };
    static TWO: [[u8; 3]; 0x7F - 0x5B] = {
        let mut t = [[0u8; 3]; 0x7F - 0x5B];
        let mut i = 0;
        while i < t.len() {
            t[i] = [0x28, 0x3F, (i + 0x5B - 0x57) as u8];
            i += 1;
        }
        t
    };
    let code = c as u32;
    if !verified_char(c) {
        return None;
    }
    if (0x21..0x5B).contains(&code) {
        Some(&ONE[(code - 0x21) as usize])
    } else {
        Some(&TWO[(code - 0x5B) as usize])
    }
}

/// Characters whose weights were checked byte for byte against native
/// indexes: digits, Latin letters, `-`, `.`, `/` and `_`. The formula covers
/// the rest of printable ASCII, but no native key proved it, so the writer
/// refuses such names instead of guessing.
fn verified_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '/' | '_')
}

/// The 392-byte key of a CONFIG-family row.
pub fn config_key(file_name: &str, part_no: u32) -> Result<Vec<u8>> {
    let count = file_name.chars().count();
    if count == 0 || count > MAX_KEY_NAME {
        return Err(Error::Refused(format!(
            "Имя строки «{file_name}»: длина {count} вне проверенного диапазона ключа индекса (1…{MAX_KEY_NAME})"
        )));
    }
    let mut key = Vec::with_capacity(CONFIG_KEY_LEN);
    for c in file_name.chars() {
        let weight = primary(c).ok_or_else(|| {
            Error::Refused(format!(
                "Имя строки «{file_name}»: символ {c:?} вне проверенной коллации ключа индекса"
            ))
        })?;
        key.extend_from_slice(weight);
    }
    key.push(0x01);
    key.push(0x04 + count as u8);
    key.push(0x00);
    key.extend_from_slice(&crate::record::pack_decimal(part_no as u64, 10));
    key.resize(CONFIG_KEY_LEN, 0);
    Ok(key)
}

/// Every `(record, key)` of the index in key order.
pub(crate) fn read_entries(object: &[u8], page_size: usize) -> Result<Vec<(u32, Vec<u8>)>> {
    let (root, key_len) = descriptor(object, page_size)?;
    let pages = object.len() / page_size;
    let mut out = Vec::new();
    let mut stack = vec![(root, 0usize)];
    let mut visited = 0usize;
    while let Some((page, depth)) = stack.pop() {
        visited += 1;
        if page as usize >= pages || visited > pages || depth > 16 {
            return Err(corrupt("Индекс: ссылка на страницу вне объекта или цикл"));
        }
        let p = &object[page as usize * page_size..(page as usize + 1) * page_size];
        let flags = u16::from_le_bytes([p[0], p[1]]);
        let count = u16::from_le_bytes([p[2], p[3]]) as usize;
        if flags & FLAG_LEAF != 0 {
            read_leaf(p, key_len, count, &mut out)?;
        } else {
            let entry = key_len + 8;
            if BRANCH_HEADER + count * entry > page_size {
                return Err(corrupt("Индекс: переполнена страница ветвления"));
            }
            // Push children right to left so the leftmost is read first.
            for k in (0..count).rev() {
                let at = BRANCH_HEADER + k * entry + key_len;
                let child = u32::from_be_bytes(p[at + 4..at + 8].try_into().unwrap());
                stack.push((child, depth + 1));
            }
        }
    }
    Ok(out)
}

fn descriptor(object: &[u8], page_size: usize) -> Result<(u32, usize)> {
    if object.len() < 2 * page_size || !object.len().is_multiple_of(page_size) {
        return Err(corrupt("Индекс: объект короче заголовка"));
    }
    let desc_page = u32::from_le_bytes(object[4..8].try_into().unwrap()) as usize;
    if desc_page == 0 || desc_page >= object.len() / page_size {
        return Err(corrupt("Индекс: нет страницы описания"));
    }
    let at = desc_page * page_size;
    let root = u32::from_le_bytes(object[at..at + 4].try_into().unwrap());
    let key_len = u16::from_le_bytes([object[at + 4], object[at + 5]]) as usize;
    if key_len == 0 || key_len > page_size / 4 {
        return Err(corrupt(format!("Индекс: длина ключа {key_len}")));
    }
    Ok((root, key_len))
}

fn read_leaf(p: &[u8], key_len: usize, count: usize, out: &mut Vec<(u32, Vec<u8>)>) -> Result<()> {
    let u16_at = |at: usize| u16::from_le_bytes([p[at], p[at + 1]]);
    let rec_mask = u32::from_le_bytes(p[14..18].try_into().unwrap()) as u64;
    let left_mask = u16_at(18) as u64;
    let right_mask = u16_at(20) as u64;
    let rec_bits = u16_at(22) as u32;
    let left_bits = u16_at(24) as u32;
    let entry = u16_at(28) as usize;
    if entry == 0 || entry > 8 || rec_bits + left_bits + u16_at(26) as u32 > 64 {
        return Err(corrupt("Индекс: неверный заголовок листа"));
    }
    if LEAF_HEADER + count * entry > p.len() {
        return Err(corrupt("Индекс: переполнен лист"));
    }
    let mut end = p.len();
    let mut prev = vec![0u8; key_len];
    for k in 0..count {
        let mut raw = [0u8; 8];
        raw[..entry].copy_from_slice(&p[LEAF_HEADER + k * entry..LEAF_HEADER + (k + 1) * entry]);
        let v = u64::from_le_bytes(raw);
        let rec = (v & rec_mask) as u32;
        let left = ((v >> rec_bits) & left_mask) as usize;
        let right = ((v >> (rec_bits + left_bits)) & right_mask) as usize;
        if left + right > key_len {
            return Err(corrupt("Индекс: сжатие ключа больше длины ключа"));
        }
        let stored = key_len - left - right;
        if end < LEAF_HEADER + count * entry + stored {
            return Err(corrupt("Индекс: ключи листа наезжают на записи"));
        }
        end -= stored;
        let mut key = Vec::with_capacity(key_len);
        key.extend_from_slice(&prev[..left]);
        key.extend_from_slice(&p[end..end + stored]);
        key.resize(key_len, 0);
        out.push((rec, key.clone()));
        prev = key;
    }
    Ok(())
}

/// Rebuilds a single-index object holding `entries` (sorted here). Page 0 and
/// the descriptor page keep their places; tree pages come from the remaining
/// pages, the unused ones are chained as free. The result is never shorter
/// than `old`.
pub(crate) fn rebuild(
    old: &[u8],
    page_size: usize,
    mut entries: Vec<(u32, Vec<u8>)>,
) -> Result<Vec<u8>> {
    let (_, key_len) = descriptor(old, page_size)?;
    if key_len != CONFIG_KEY_LEN {
        return Err(Error::Refused(format!(
            "Индекс с ключом {key_len} байт: запись поддержана только для таблиц семейства CONFIG"
        )));
    }
    if u32::from_le_bytes(old[8..12].try_into().unwrap()) != 0 {
        return Err(Error::Refused(
            "Индексный объект с несколькими индексами: запись поддержана только для таблиц семейства CONFIG".into(),
        ));
    }
    let desc_page = u32::from_le_bytes(old[4..8].try_into().unwrap()) as usize;
    entries.sort_by(|a, b| a.1.cmp(&b.1));
    if entries.windows(2).any(|w| w[0].1 == w[1].1) {
        return Err(Error::Refused(
            "Индекс: повторяющийся ключ (имя и номер части)".into(),
        ));
    }

    // Leaves are filled to half a page, as the platform's splits leave them:
    // deleting an entry can grow the next one (its shared prefix is lost), so
    // a full leaf would overflow when the platform empties the table.
    let budget = (page_size - LEAF_HEADER) / 2;
    let mut leaves: Vec<Vec<LeafEntry>> = Vec::new();
    let mut current: Vec<LeafEntry> = Vec::new();
    let mut free = budget;
    // `left` shares bytes only with the significant part of the previous key
    // (`key_len - right` of it): the platform rebuilds keys in a buffer that
    // holds just that part, and a longer `left` (two parts of one row share
    // zero bytes of PARTNO) makes it read and write past the buffer.
    let mut prev: Option<(&[u8], usize)> = None;
    for (rec, key) in &entries {
        let left = prev.map_or(0, |(p, p_right)| {
            common_prefix(p, key).min(key_len - p_right)
        });
        let right = key.iter().rev().take_while(|b| **b == 0).count();
        let right = right.min(key_len - left);
        let need = ENTRY_BYTES as usize + (key_len - left - right);
        if need > free && !current.is_empty() {
            leaves.push(std::mem::take(&mut current));
            free = budget;
            // The first key of a page is stored without a shared prefix.
            let right = key.iter().rev().take_while(|b| **b == 0).count();
            current.push((*rec, key.clone(), 0, right));
            free -= ENTRY_BYTES as usize + (key_len - right);
            prev = Some((key, right));
        } else {
            current.push((*rec, key.clone(), left, right));
            free -= need;
            prev = Some((key, right));
        }
    }
    leaves.push(current);

    // Page allocation: every page except 0 and the descriptor, in order.
    let old_pages = old.len() / page_size;
    let mut pool: Vec<usize> = (1..old_pages).filter(|p| *p != desc_page).collect();
    pool.reverse();
    let mut next_new = old_pages;
    let mut alloc = || {
        pool.pop().unwrap_or_else(|| {
            next_new += 1;
            next_new - 1
        })
    };

    let mut pages: Vec<(usize, Vec<u8>)> = Vec::new();
    let leaf_numbers: Vec<usize> = leaves.iter().map(|_| alloc()).collect();
    let single = leaves.len() == 1;
    let mut level: Vec<(usize, u32, Vec<u8>)> = Vec::new(); // (page, last rec, last key)
    for (i, leaf) in leaves.iter().enumerate() {
        let prev_page = if i == 0 {
            NONE
        } else {
            leaf_numbers[i - 1] as u32
        };
        let next_page = if i + 1 == leaves.len() {
            NONE
        } else {
            leaf_numbers[i + 1] as u32
        };
        let flags = FLAG_LEAF | if single { FLAG_ROOT } else { 0 };
        pages.push((
            leaf_numbers[i],
            leaf_page(page_size, key_len, flags, prev_page, next_page, leaf),
        ));
        if let Some(last) = leaf.last() {
            level.push((leaf_numbers[i], last.0, last.1.clone()));
        }
    }
    let mut root = leaf_numbers[0];
    // Branch pages are half full too, leaving room for the platform's splits.
    let per_branch = ((page_size - BRANCH_HEADER) / (key_len + 8) / 2 + 1).max(2);
    while level.len() > 1 {
        let groups: Vec<&[(usize, u32, Vec<u8>)]> = level.chunks(per_branch).collect();
        let numbers: Vec<usize> = groups.iter().map(|_| alloc()).collect();
        let top = groups.len() == 1;
        let mut upper = Vec::new();
        for (i, group) in groups.iter().enumerate() {
            let mut p = vec![0u8; page_size];
            let flags = if top { FLAG_ROOT } else { 0 };
            p[0..2].copy_from_slice(&flags.to_le_bytes());
            p[2..4].copy_from_slice(&(group.len() as u16).to_le_bytes());
            let prev_page = if i == 0 { NONE } else { numbers[i - 1] as u32 };
            let next_page = if i + 1 == groups.len() {
                NONE
            } else {
                numbers[i + 1] as u32
            };
            p[4..8].copy_from_slice(&prev_page.to_le_bytes());
            p[8..12].copy_from_slice(&next_page.to_le_bytes());
            for (k, (child, rec, key)) in group.iter().enumerate() {
                let at = BRANCH_HEADER + k * (key_len + 8);
                p[at..at + key_len].copy_from_slice(key);
                p[at + key_len..at + key_len + 4].copy_from_slice(&rec.to_be_bytes());
                p[at + key_len + 4..at + key_len + 8]
                    .copy_from_slice(&(*child as u32).to_be_bytes());
            }
            pages.push((numbers[i], p));
            let last = group.last().unwrap();
            upper.push((numbers[i], last.1, last.2.clone()));
        }
        root = numbers[0];
        level = upper;
    }

    let total = next_new.max(old_pages);
    let mut out = vec![0u8; total * page_size];
    out[..page_size].copy_from_slice(&old[..page_size]);
    out[desc_page * page_size..(desc_page + 1) * page_size]
        .copy_from_slice(&old[desc_page * page_size..(desc_page + 1) * page_size]);
    out[desc_page * page_size..desc_page * page_size + 4]
        .copy_from_slice(&(root as u32).to_le_bytes());
    for (page, content) in &pages {
        out[page * page_size..(page + 1) * page_size].copy_from_slice(content);
    }
    // Remaining pool pages become the free chain (ascending).
    let mut free_pages: Vec<usize> = pool;
    free_pages.sort_unstable();
    let head = free_pages.first().map_or(0, |p| *p as u32);
    out[0..4].copy_from_slice(&head.to_le_bytes());
    for (i, page) in free_pages.iter().enumerate() {
        let next = free_pages.get(i + 1).map_or(0, |p| *p as u32);
        let at = page * page_size;
        out[at..at + page_size].fill(0);
        out[at..at + 4].copy_from_slice(&next.to_le_bytes());
    }
    Ok(out)
}

/// A leaf entry being written: record, key, `left` and `right`.
type LeafEntry = (u32, Vec<u8>, usize, usize);

fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}

fn leaf_page(
    page_size: usize,
    key_len: usize,
    flags: u16,
    prev: u32,
    next: u32,
    entries: &[LeafEntry],
) -> Vec<u8> {
    let mut p = vec![0u8; page_size];
    p[0..2].copy_from_slice(&flags.to_le_bytes());
    p[2..4].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    p[4..8].copy_from_slice(&prev.to_le_bytes());
    p[8..12].copy_from_slice(&next.to_le_bytes());
    p[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
    p[18..20].copy_from_slice(&u16::MAX.to_le_bytes());
    p[20..22].copy_from_slice(&u16::MAX.to_le_bytes());
    p[22..24].copy_from_slice(&REC_BITS.to_le_bytes());
    p[24..26].copy_from_slice(&LEFT_BITS.to_le_bytes());
    p[26..28].copy_from_slice(&RIGHT_BITS.to_le_bytes());
    p[28..30].copy_from_slice(&ENTRY_BYTES.to_le_bytes());
    let mut end = page_size;
    for (k, (rec, key, left, right)) in entries.iter().enumerate() {
        let v = (*rec as u64)
            | ((*left as u64) << REC_BITS)
            | ((*right as u64) << (REC_BITS + LEFT_BITS));
        let at = LEAF_HEADER + k * ENTRY_BYTES as usize;
        p[at..at + 8].copy_from_slice(&v.to_le_bytes());
        let stored = &key[*left..key_len - right];
        end -= stored.len();
        p[end..end + stored.len()].copy_from_slice(stored);
    }
    let free = end - (LEAF_HEADER + entries.len() * ENTRY_BYTES as usize);
    p[12..14].copy_from_slice(&(free as u16).to_le_bytes());
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_of_native_rows_match_the_measured_encoding() {
        // Key heads copied from native 8.5.1.1150 PARAMS/CONFIG indexes.
        let k = config_key("DBNames", 0).unwrap();
        assert_eq!(
            &k[..22],
            &[
                0x28, 0x28, 0x28, 0x26, 0x28, 0x32, 0x28, 0x3f, 0x0a, 0x28, 0x3f, 0x16, 0x28, 0x3f,
                0x0e, 0x28, 0x3f, 0x1c, 0x01, 0x0b, 0x00, 0x10
            ]
        );
        let k = config_key("0012c33d-4e24-47a2-887e-6164d9a5076f.0", 1).unwrap();
        let tail_at = k.iter().position(|b| *b == 0x01).unwrap();
        assert_eq!(
            &k[tail_at..tail_at + 9],
            &[0x01, 0x2a, 0x00, 0x10, 0, 0, 0, 0, 0x10]
        );
        assert_eq!(k.len(), CONFIG_KEY_LEN);
    }

    #[test]
    fn key_order_is_code_point_order() {
        let names = [
            "0b6",
            "5927",
            "DBNames",
            "DBNamesVersion-DBNames",
            "a07b",
            "ibparams.inf",
            "log.inf",
        ];
        let keys: Vec<Vec<u8>> = names.iter().map(|n| config_key(n, 0).unwrap()).collect();
        assert!(keys.windows(2).all(|w| w[0] < w[1]));
        assert!(config_key("a", 0).unwrap() < config_key("a", 1).unwrap());
    }

    #[test]
    fn unverified_names_are_refused() {
        assert!(config_key("с кириллицей", 0).is_err());
        assert!(config_key("with space", 0).is_err());
        assert!(config_key(&"x".repeat(44), 0).is_err());
        assert!(config_key("", 0).is_err());
    }

    fn empty_index(page_size: usize) -> Vec<u8> {
        let mut obj = vec![0u8; 3 * page_size];
        obj[4..8].copy_from_slice(&1u32.to_le_bytes());
        let d = page_size;
        obj[d..d + 4].copy_from_slice(&2u32.to_le_bytes());
        obj[d + 4..d + 6].copy_from_slice(&(CONFIG_KEY_LEN as u16).to_le_bytes());
        obj[d + 6..d + 8].copy_from_slice(&0x80u16.to_le_bytes());
        let leaf = leaf_page(page_size, CONFIG_KEY_LEN, 3, NONE, NONE, &[]);
        obj[2 * page_size..3 * page_size].copy_from_slice(&leaf);
        obj
    }

    #[test]
    fn rebuilt_trees_read_back_in_order() {
        let ps = 8192;
        for n in [0usize, 1, 50, 3000] {
            let entries: Vec<(u32, Vec<u8>)> = (0..n)
                .map(|i| {
                    (
                        i as u32 + 1,
                        config_key(&format!("{:08x}-row.{}", i * 7919 % 100_003, i % 3), 0)
                            .unwrap(),
                    )
                })
                .collect();
            let obj = rebuild(&empty_index(ps), ps, entries.clone()).unwrap();
            let mut expected = entries;
            expected.sort_by(|a, b| a.1.cmp(&b.1));
            assert_eq!(read_entries(&obj, ps).unwrap(), expected, "n = {n}");
            assert!(obj.len() >= 3 * ps);
        }
    }

    #[test]
    fn parts_of_one_row_share_only_the_significant_prefix() {
        // Native leaves store part 1 after part 0 with left = 93 (the end of
        // part 0's significant bytes), not the 97 bytes the two keys share.
        let name = "a2782760-dd4c-44d2-a785-814d6f610455.0";
        let entries = vec![
            (1, config_key(name, 0).unwrap()),
            (2, config_key(name, 1).unwrap()),
        ];
        let obj = rebuild(&empty_index(8192), 8192, entries.clone()).unwrap();
        let leaf = &obj[2 * 8192..3 * 8192];
        let v = u64::from_le_bytes(leaf[LEAF_HEADER + 8..LEAF_HEADER + 16].try_into().unwrap());
        let left = ((v >> 32) & 0xFFFF) as usize;
        let p0_right = entries[0].1.iter().rev().take_while(|b| **b == 0).count();
        assert_eq!(left, CONFIG_KEY_LEN - p0_right);
        assert_eq!(read_entries(&obj, 8192).unwrap(), entries);
    }

    #[test]
    fn duplicate_keys_are_refused() {
        let k = config_key("root", 0).unwrap();
        assert!(rebuild(&empty_index(8192), 8192, vec![(1, k.clone()), (2, k)]).is_err());
    }
}
