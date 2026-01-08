use crate::FXY;
use rkyv::Archive;
use rkyv::api::high::{HighDeserializer, HighSerializer, HighValidator};
use rkyv::bytecheck::CheckBytes;
use rkyv::de::Pool;
use rkyv::rancor::{Error, Strategy};
use serde::Serialize as SerdeSerialize;
use serde::de::DeserializeOwned;
use std::fmt::{Debug, Display};
use toml::Table;

pub struct BTable;
pub struct DTable;

pub struct BitMap;

pub trait TableTypeTrait
where
    <Self::EntryType as Archive>::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>,
{
    type EntryType: TableEntryFull;
    const TABLE_TYPE: crate::TableType;
}

impl TableTypeTrait for BTable {
    type EntryType = crate::tables::BTableEntry;
    const TABLE_TYPE: crate::TableType = crate::TableType::B;
}
impl TableTypeTrait for DTable {
    type EntryType = crate::tables::DTableEntry;
    const TABLE_TYPE: crate::TableType = crate::TableType::D;
}

impl TableTypeTrait for BitMap {
    type EntryType = crate::tables::BitMapEntry;
    const TABLE_TYPE: crate::TableType = crate::TableType::BitMap;
}

pub trait TableEntry:
    SerdeSerialize
    + DeserializeOwned
    + std::fmt::Display
    + Debug
    + Clone
    + Sized
    + Archive
    + for<'a> rkyv::Serialize<
        HighSerializer<rkyv::util::AlignedVec, rkyv::ser::allocator::ArenaHandle<'a>, Error>,
    >
{
    fn fxy(&self) -> FXY;
}

// 148 |     fn get(&self, fxy: FXY) -> Option<T> where for<'a> <T as TableEntryFull>::Archived: CheckBytes<Strategy<Validator<ArchiveValidator<'a>, SharedValidator>, rkyv::rancor::Error>>

pub trait TableEntryFull: TableEntry {
    type Archived: for<'a> rkyv::Deserialize<Self, HighDeserializer<Error>>
        + rkyv::Deserialize<Self, Strategy<Pool, rkyv::rancor::Error>>
        + rkyv::Portable
        + std::fmt::Display
        + for<'a> CheckBytes<HighValidator<'a, Error>>;
}

impl<T> TableEntryFull for T
where
    T: TableEntry,
    <T as Archive>::Archived: for<'a> rkyv::Deserialize<T, HighDeserializer<Error>>
        + rkyv::Deserialize<T, Strategy<Pool, rkyv::rancor::Error>>
        + std::fmt::Display
        + for<'a> CheckBytes<HighValidator<'a, Error>>,
{
    type Archived = <T as Archive>::Archived;
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct BTableEntry {
    pub fxy: FXY,
    pub class_name_en: String,
    pub element_name_en: String,
    pub bufr_unit: String,
    pub bufr_scale: i32,
    pub bufr_reference_value: i32,
    pub bufr_datawidth_bits: u32,
    pub note_en: Option<String>,
    pub note_ids: Option<String>,
    pub status: Option<String>,
}

impl BTableEntry {
    pub fn fxy(&self) -> FXY {
        self.fxy
    }

    pub fn class_name_en(&self) -> &str {
        &self.class_name_en
    }

    pub fn element_name_en(&self) -> &str {
        &self.element_name_en
    }

    pub fn bufr_unit(&self) -> &str {
        &self.bufr_unit
    }

    pub fn bufr_scale(&self) -> i32 {
        self.bufr_scale
    }

    pub fn bufr_reference_value(&self) -> i32 {
        self.bufr_reference_value
    }

    pub fn bufr_datawidth_bits(&self) -> u32 {
        self.bufr_datawidth_bits
    }

    pub fn note_en(&self) -> Option<&str> {
        self.note_en.as_deref()
    }

    pub fn note_ids(&self) -> Option<&str> {
        self.note_ids.as_deref()
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }
}

impl Display for BTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let element_name = if self.element_name_en.len() > 40 {
            format!("{}...", &self.element_name_en[..37])
        } else {
            self.element_name_en.clone()
        };

        let unit = if self.bufr_unit.len() > 15 {
            format!("{}...", &self.bufr_unit[..12])
        } else {
            self.bufr_unit.clone()
        };

        write!(
            f,
            "{:02}{:02}{:03} | {:<40} | {:<15} | {:>5} | {:>8} | {:>8} | {}",
            self.fxy.f,
            self.fxy.x,
            self.fxy.y,
            element_name,
            unit,
            self.bufr_scale,
            self.bufr_reference_value,
            self.bufr_datawidth_bits,
            self.status().unwrap_or("N/A")
        )
    }
}

impl Display for ArchivedBTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let element_name = if self.element_name_en.len() > 40 {
            format!("{}...", &self.element_name_en[..37])
        } else {
            self.element_name_en.to_string()
        };

        let unit = if self.bufr_unit.len() > 15 {
            format!("{}...", &self.bufr_unit[..12])
        } else {
            self.bufr_unit.to_string()
        };

        write!(
            f,
            "{:02}{:02}{:03} | {:<40} | {:<15} | {:>5} | {:>8} | {:>8} | {}",
            self.fxy.f,
            self.fxy.x,
            self.fxy.y,
            element_name,
            unit,
            self.bufr_scale,
            self.bufr_reference_value,
            self.bufr_datawidth_bits,
            self.status.as_deref().unwrap_or("N/A")
        )
    }
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct DTableEntry {
    pub fxy: FXY,
    pub fxy_chain: Vec<FXY>,
    pub category: Option<String>,
    pub category_of_sequences_en: Option<String>,
    pub title_en: Option<String>,
    pub subtitle_en: Option<String>,
    pub note_en: Option<String>,
    pub note_ids: Option<String>,
    pub status: Option<String>,
}

impl DTableEntry {
    pub fn fxy(&self) -> FXY {
        self.fxy
    }

    pub fn fxy_chain(&self) -> &[FXY] {
        &self.fxy_chain
    }

    pub fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }

    pub fn category_of_sequences_en(&self) -> Option<&str> {
        self.category_of_sequences_en.as_deref()
    }

    pub fn title_en(&self) -> Option<&str> {
        self.title_en.as_deref()
    }

    pub fn subtitle_en(&self) -> Option<&str> {
        self.subtitle_en.as_deref()
    }

    pub fn note_en(&self) -> Option<&str> {
        self.note_en.as_deref()
    }

    pub fn note_ids(&self) -> Option<&str> {
        self.note_ids.as_deref()
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }
}

impl std::fmt::Display for DTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fxy_chain_str: String = self
            .fxy_chain
            .iter()
            .map(|fxy| format!("{:02}{:02}{:03}", fxy.f, fxy.x, fxy.y))
            .collect::<Vec<_>>()
            .join(", ");

        let title = self.title_en.as_deref().unwrap_or("N/A");
        let truncated_title = if title.len() > 50 {
            format!("{}...", &title[..47])
        } else {
            title.to_string()
        };

        write!(
            f,
            "{:02}{:02}{:03} | {:<50} | {:<12} | [{}]",
            self.fxy.f,
            self.fxy.x,
            self.fxy.y,
            truncated_title,
            self.status().unwrap_or("N/A"),
            fxy_chain_str
        )
    }
}

impl Display for ArchivedDTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fxy_chain_str: String = self
            .fxy_chain
            .iter()
            .map(|fxy| format!("{:02}{:02}{:03}", fxy.f, fxy.x, fxy.y))
            .collect::<Vec<_>>()
            .join(", ");

        let title = self.title_en.as_deref().unwrap_or("N/A");
        let truncated_title = if title.len() > 50 {
            format!("{}...", &title[..47])
        } else {
            title.to_string()
        };

        write!(
            f,
            "{:02}{:02}{:03} | {:<50} | {:<12} | [{}]",
            self.fxy.f,
            self.fxy.x,
            self.fxy.y,
            truncated_title,
            self.status.as_deref().unwrap_or("N/A"),
            fxy_chain_str
        )
    }
}

#[derive(
    Debug, Clone, serde::Deserialize, serde::Serialize, Archive, rkyv::Serialize, rkyv::Deserialize,
)]
#[rkyv(compare(PartialEq), derive(Debug))]
pub struct BitMapEntry {
    pub fxy: FXY,
    pub depth: u8,
}

impl Display for BitMapEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02}{:02}{:03} | Depth: {}",
            self.fxy.f, self.fxy.x, self.fxy.y, self.depth
        )
    }
}

impl Display for ArchivedBitMapEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02}{:02}{:03} | Depth: {}",
            self.fxy.f, self.fxy.x, self.fxy.y, self.depth
        )
    }
}

impl TableEntry for BitMapEntry {
    fn fxy(&self) -> FXY {
        self.fxy
    }
}

impl TableEntry for DTableEntry {
    fn fxy(&self) -> FXY {
        self.fxy
    }
}

impl TableEntry for BTableEntry {
    fn fxy(&self) -> FXY {
        self.fxy
    }
}
