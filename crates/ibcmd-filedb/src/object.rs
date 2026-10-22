//! Objects: the byte streams that every table part (records, blobs, index)
//! and the root table list are stored in.
//!
//! An object header page (format 8.3.8):
//!
//! | offset | size | value                                                  |
//! |--------|------|--------------------------------------------------------|
//! | 0      | 2    | signature `0xFD1C` (`0xFF1C` for the free-pages object)|
//! | 2      | 2    | fat level: 0 — the list holds data pages, 1 — the list |
//! |        |      | holds pages of `page_size/4` data page numbers         |
//! | 4      | 4    | change counter                                         |
//! | 8      | 4    | length-change counter                                  |
//! | 12     | 4    | number of second-level pages when the fat level is 1   |
//! | 16     | 8    | object length in bytes                                 |
//! | 24     | …    | page numbers, `(page_size - 24) / 4` of them, 0 = none |

use crate::error::{Result, corrupt};
use crate::pager::Pager;

pub(crate) const OBJECT_SIGNATURE: u16 = 0xFD1C;
const HEADER: usize = 24;

pub(crate) struct Object {
    pub(crate) header_page: u32,
    fat: u16,
    counter: u32,
    length_counter: u32,
    pub(crate) len: u64,
    /// Data pages in object order.
    pub(crate) pages: Vec<u32>,
    /// Second-level pages when the fat level is 1.
    tables: Vec<u32>,
}

fn direct_capacity(page_size: usize) -> usize {
    (page_size - HEADER) / 4
}

impl Object {
    pub(crate) fn open(pager: &mut Pager<'_>, header_page: u32) -> Result<Object> {
        pager.check_page(header_page, "Заголовок объекта")?;
        let page = pager.read_page(header_page)?;
        let sig = u16::from_le_bytes([page[0], page[1]]);
        if sig != OBJECT_SIGNATURE {
            return Err(corrupt(format!(
                "Страница {header_page} не заголовок объекта: сигнатура {sig:#06x}"
            )));
        }
        let fat = u16::from_le_bytes([page[2], page[3]]);
        let counter = u32::from_le_bytes(page[4..8].try_into().unwrap());
        let length_counter = u32::from_le_bytes(page[8..12].try_into().unwrap());
        let len = u64::from_le_bytes(page[16..24].try_into().unwrap());
        let ps = pager.page_size() as u64;
        // An object cannot hold more pages than the file has: a corrupted
        // length is refused before anything is allocated for it.
        let need = len.div_ceil(ps);
        if need > pager.page_count() as u64 {
            return Err(corrupt(format!(
                "Объект {header_page}: длина {len} больше файла из {} страниц",
                pager.page_count()
            )));
        }
        let need = need as usize;
        let direct: Vec<u32> = page[HEADER..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect();
        let mut pages = Vec::with_capacity(need);
        let mut tables = Vec::new();
        match fat {
            0 => {
                if need > direct.len() {
                    return Err(corrupt(format!(
                        "Объект {header_page}: длина {len} не помещается в одноуровневую таблицу размещения"
                    )));
                }
                pages.extend_from_slice(&direct[..need]);
            }
            1 => {
                let per = pager.page_size() / 4;
                let table_count = need.div_ceil(per);
                if table_count > direct.len() {
                    return Err(corrupt(format!("Объект {header_page}: слишком длинный")));
                }
                for &table in &direct[..table_count] {
                    pager.check_page(table, "Таблица размещения")?;
                    tables.push(table);
                    let list = pager.read_page(table)?;
                    for chunk in list.as_chunks::<4>().0 {
                        if pages.len() == need {
                            break;
                        }
                        pages.push(u32::from_le_bytes(*chunk));
                    }
                }
            }
            other => {
                return Err(corrupt(format!(
                    "Объект {header_page}: неизвестный уровень таблицы размещения {other}"
                )));
            }
        }
        for &p in &pages {
            pager.check_page(p, "Страница данных объекта")?;
        }
        Ok(Object {
            header_page,
            fat,
            counter,
            length_counter,
            len,
            pages,
            tables,
        })
    }

    /// Reads `buf.len()` bytes at `offset` of the object.
    pub(crate) fn read_at(&self, pager: &mut Pager<'_>, offset: u64, buf: &mut [u8]) -> Result<()> {
        let end = offset
            .checked_add(buf.len() as u64)
            .filter(|end| *end <= self.len)
            .ok_or_else(|| {
                corrupt(format!(
                    "Чтение {}..+{} за концом объекта {} длиной {}",
                    offset,
                    buf.len(),
                    self.header_page,
                    self.len
                ))
            })?;
        let ps = pager.page_size() as u64;
        let mut page_buf = vec![0u8; ps as usize];
        let mut pos = offset;
        let mut done = 0usize;
        let mut cached: Option<usize> = None;
        while pos < end {
            let index = (pos / ps) as usize;
            let within = (pos % ps) as usize;
            if cached != Some(index) {
                pager.read_page_into(self.pages[index], &mut page_buf)?;
                cached = Some(index);
            }
            let take = ((ps as usize) - within).min((end - pos) as usize);
            buf[done..done + take].copy_from_slice(&page_buf[within..within + take]);
            done += take;
            pos += take as u64;
        }
        Ok(())
    }

    pub(crate) fn read_all(&self, pager: &mut Pager<'_>) -> Result<Vec<u8>> {
        let len = usize::try_from(self.len)
            .map_err(|_| corrupt(format!("Объект {} слишком велик", self.header_page)))?;
        pager.materialize_object(self.len)?;
        let mut buf = vec![0u8; len];
        self.read_at(pager, 0, &mut buf)?;
        Ok(buf)
    }

    /// Replaces the whole content. The object never shrinks: the caller keeps
    /// the space it no longer uses inside the content (free record and free
    /// block chains), as the platform itself does after `config apply`.
    /// Existing data pages are reused in order, missing ones are appended at
    /// the end of the file.
    pub(crate) fn rewrite(&mut self, pager: &mut Pager<'_>, content: &[u8]) -> Result<()> {
        if (content.len() as u64) < self.len {
            return Err(crate::error::Error::Refused(format!(
                "Внутренняя ошибка: объект {} укорачивается с {} до {} байт",
                self.header_page,
                self.len,
                content.len()
            )));
        }
        let ps = pager.page_size();
        let need = content.len().div_ceil(ps);
        while self.pages.len() < need {
            let page = pager.append_page()?;
            self.pages.push(page);
        }
        let mut page_buf = vec![0u8; ps];
        for (index, chunk) in content.chunks(ps).enumerate() {
            page_buf[..chunk.len()].copy_from_slice(chunk);
            page_buf[chunk.len()..].fill(0);
            pager.write_page(self.pages[index], &page_buf)?;
        }
        let old_len = self.len;
        self.len = content.len() as u64;
        self.counter = self.counter.wrapping_add(1);
        if self.len != old_len {
            self.length_counter = self.length_counter.wrapping_add(1);
        }
        self.write_header(pager)
    }

    /// Every page the object owns: its header, second-level and data pages.
    pub(crate) fn owned_pages(&self) -> impl Iterator<Item = u32> + '_ {
        std::iter::once(self.header_page)
            .chain(self.tables.iter().copied())
            .chain(self.pages.iter().copied())
    }

    fn write_header(&mut self, pager: &mut Pager<'_>) -> Result<()> {
        let ps = pager.page_size();
        let direct = direct_capacity(ps);
        let need = (self.len.div_ceil(ps as u64)) as usize;
        let pages = &self.pages[..need];
        let mut header = vec![0u8; ps];
        header[0..2].copy_from_slice(&OBJECT_SIGNATURE.to_le_bytes());
        if pages.len() <= direct && self.fat == 0 {
            for (i, p) in pages.iter().enumerate() {
                header[HEADER + i * 4..HEADER + i * 4 + 4].copy_from_slice(&p.to_le_bytes());
            }
            header[12..16].copy_from_slice(&0u32.to_le_bytes());
        } else {
            self.fat = 1;
            let per = ps / 4;
            let table_count = pages.len().div_ceil(per);
            if table_count > direct {
                return Err(crate::error::Error::Refused(
                    "Объект превышает предельный размер двухуровневой таблицы размещения".into(),
                ));
            }
            while self.tables.len() < table_count {
                let page = pager.append_page()?;
                self.tables.push(page);
            }
            for (t, chunk) in pages.chunks(per).enumerate() {
                let mut list = vec![0u8; ps];
                for (i, p) in chunk.iter().enumerate() {
                    list[i * 4..i * 4 + 4].copy_from_slice(&p.to_le_bytes());
                }
                pager.write_page(self.tables[t], &list)?;
                header[HEADER + t * 4..HEADER + t * 4 + 4]
                    .copy_from_slice(&self.tables[t].to_le_bytes());
            }
            header[12..16].copy_from_slice(&(table_count as u32).to_le_bytes());
        }
        header[2..4].copy_from_slice(&self.fat.to_le_bytes());
        header[4..8].copy_from_slice(&self.counter.to_le_bytes());
        header[8..12].copy_from_slice(&self.length_counter.to_le_bytes());
        header[16..24].copy_from_slice(&self.len.to_le_bytes());
        pager.write_page(self.header_page, &header)
    }
}
