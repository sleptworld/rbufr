use super::EntryLoader;
use librbufr::core::{
    FXY,
    tables::{DTable, DTableEntry},
};

#[derive(Debug, Clone, Default)]
pub struct DTableCsvLoader {
    current_chain: Option<DTableEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RawDTableEntry {
    #[serde(rename = "Category")]
    pub category: Option<String>,
    #[serde(rename = "CategoryOfSequences_en")]
    pub category_of_sequences_en: Option<String>,
    #[serde(rename = "FXY1")]
    pub fxy1: String,
    #[serde(rename = "Title_en")]
    pub title_en: Option<String>,
    #[serde(rename = "SubTitle_en")]
    pub subtitle_en: Option<String>,
    #[serde(rename = "FXY2")]
    pub fxy2: String,
    #[serde(rename = "ElementName_en")]
    pub _element_name_en: Option<String>,
    #[serde(rename = "ElementDescription_en")]
    pub _element_description_en: Option<String>,
    #[serde(rename = "Note_en")]
    pub note_en: Option<String>,
    #[serde(rename = "noteIDs")]
    pub note_ids: Option<String>,
    #[serde(rename = "Status")]
    pub status: Option<String>,
}

impl EntryLoader for DTableCsvLoader {
    type RawEntry = RawDTableEntry;
    type Output = DTableEntry;
    type TableType = DTable;

    fn process_entry(&mut self, raw: Self::RawEntry) -> anyhow::Result<Option<Self::Output>> {
        // Process the raw entry as needed
        if self.current_chain.is_none() {
            let entry = DTableEntry {
                fxy: FXY::from_str(&raw.fxy1)?,
                fxy_chain: vec![FXY::from_str(&raw.fxy2)?],
                category: raw.category,
                category_of_sequences_en: raw.category_of_sequences_en,
                title_en: raw.title_en,
                subtitle_en: raw.subtitle_en,
                note_en: raw.note_en,
                note_ids: raw.note_ids,
                status: raw.status,
            };
            self.current_chain = Some(entry);
            return Ok(None);
        } else {
            let fxy = FXY::from_str(&raw.fxy1)?;
            if self.current_chain.as_ref().unwrap().fxy != fxy {
                // First take out the old completed chain
                let finished = self.current_chain.take();

                // Then create and save the new chain
                let entry = DTableEntry {
                    fxy,
                    fxy_chain: vec![FXY::from_str(&raw.fxy2)?],
                    category: raw.category,
                    category_of_sequences_en: raw.category_of_sequences_en,
                    title_en: raw.title_en,
                    subtitle_en: raw.subtitle_en,
                    note_en: raw.note_en,
                    note_ids: raw.note_ids,
                    status: raw.status,
                };
                self.current_chain = Some(entry);

                // Return the old completed chain
                return Ok(finished);
            } else {
                self.current_chain
                    .as_mut()
                    .unwrap()
                    .fxy_chain
                    .push(FXY::from_str(&raw.fxy2)?);

                return Ok(None);
            }
        }
    }

    fn finish(&mut self) -> anyhow::Result<Option<Self::Output>> {
        Ok(self.current_chain.take())
    }
}
