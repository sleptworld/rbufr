use super::EntryLoader;
use librbufr::core::{
    FXY,
    tables::{BTable, BTableEntry},
};

#[derive(Default)]
pub struct BTableCsvLoader;

#[derive(Debug, serde::Deserialize)]
pub struct RawBTableEntry {
    #[serde(rename = "ClassNo")]
    pub class_no: String,
    #[serde(rename = "ClassName_en")]
    pub class_name_en: String,
    #[serde(rename = "FXY")]
    pub fxy: String,
    #[serde(rename = "ElementName_en")]
    pub element_name_en: String,
    #[serde(rename = "BUFR_Unit")]
    pub bufr_unit: String,
    #[serde(rename = "BUFR_Scale")]
    pub bufr_scale: i32,
    #[serde(rename = "BUFR_ReferenceValue")]
    pub bufr_reference_value: i32,
    #[serde(rename = "BUFR_DataWidth_Bits")]
    pub bufr_datawidth_bits: u32,
    #[serde(rename = "CREX_Unit")]
    pub crex_unit: Option<String>,
    #[serde(rename = "CREX_Scale")]
    pub crex_scale: Option<i32>,
    #[serde(rename = "CREX_DataWidth_Char")]
    pub crex_datawidth_char: Option<u32>,
    #[serde(rename = "Note_en")]
    pub note_en: Option<String>,
    #[serde(rename = "noteIDs")]
    pub note_ids: Option<String>,
    #[serde(rename = "Status")]
    pub status: Option<String>,
}

impl EntryLoader for BTableCsvLoader {
    type RawEntry = RawBTableEntry;
    type Output = BTableEntry;
    type TableType = BTable;

    fn process_entry(&mut self, raw: Self::RawEntry) -> anyhow::Result<Option<Self::Output>> {
        let fxy = FXY::from_str(&raw.fxy)?;

        let entry = BTableEntry {
            fxy,
            class_name_en: raw.class_name_en,
            element_name_en: raw.element_name_en,
            bufr_unit: raw.bufr_unit,
            bufr_scale: raw.bufr_scale,
            bufr_reference_value: raw.bufr_reference_value,
            bufr_datawidth_bits: raw.bufr_datawidth_bits,
            note_en: raw.note_en,
            note_ids: raw.note_ids,
            status: raw.status,
        };

        Ok(Some(entry))
    }
}
