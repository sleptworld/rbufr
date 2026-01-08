use super::EntryLoader;
use crate::{
    FXY,
    tables::{BTable, BTableEntry},
};

#[derive(Default)]
pub struct BTableLoader;

#[derive(Debug)]
pub struct RawBTableEntry {
    pub f: u16,
    pub x: u16,
    pub y: u16,
}

impl EntryLoader for BTableLoader {
    type Output = BTableEntry;
    type TableType = BTable;

    fn process_entry(&mut self, raw: csv::StringRecord) -> anyhow::Result<Option<Self::Output>> {
        let parse_num_field = |index: usize| {
            raw.get(index)
                .map(|s| {
                    let mut s = s.to_string();
                    s.retain(|c| {
                        c.is_alphanumeric()
                            || c == '-'
                            || c == '.'
                            || c == '+'
                            || c == 'e'
                            || c == 'E'
                            || c == '_'
                            || c == '/'
                    });
                    s
                })
                .ok_or_else(|| anyhow::anyhow!("Missing field at index {}", index))
        };

        let parse_field = |index: usize| {
            raw.get(index)
                .map(|s| {
                    let s = s.to_string();
                    s
                })
                .ok_or_else(|| anyhow::anyhow!("Missing field at index {}", index))
        };

        let f = parse_num_field(0)?.parse()?;
        let x = parse_num_field(1)?.parse()?;
        let y = parse_num_field(2)?.parse()?;

        let fxy = FXY::new(f, x, y);

        let class_name_en = parse_field(3)?;
        let bufr_unit = parse_field(4)?;
        let bufr_scale = parse_num_field(5)?.parse()?;
        let bufr_reference_value = parse_num_field(6)?.parse()?;
        let bufr_datawidth_bits = parse_num_field(7)?.parse()?;

        let entry = BTableEntry {
            fxy,
            class_name_en: class_name_en.clone(),
            element_name_en: class_name_en,
            bufr_unit,
            bufr_scale,
            bufr_reference_value,
            bufr_datawidth_bits,
            note_en: None,
            note_ids: None,
            status: None,
        };

        Ok(Some(entry))
    }
}
