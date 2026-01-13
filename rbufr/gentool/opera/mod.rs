use librbufr::core::{
    FXY, TableConverter,
    tables::{BitMap, BitMapEntry},
};

use csv::ReaderBuilder;

pub struct TableLoader {}

impl TableConverter for TableLoader {
    type OutputEntry = BitMapEntry;
    type TableType = BitMap;

    fn convert<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> anyhow::Result<Vec<Self::OutputEntry>> {
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b';')
            .flexible(true) // Allow variable number of fields
            .from_path(path.as_ref())?;

        let mut entries = vec![];

        for result in rdr.records() {
            let record = result?;

            let parse_field = |idx: usize| {
                record
                    .get(idx)
                    .map(|s| s.trim().to_string())
                    .ok_or_else(|| {
                        anyhow::anyhow!("Parse Opera Bitmap File failed at index {}", idx)
                    })
            };

            let f = parse_field(0)?.parse()?;
            let x = parse_field(1)?.parse()?;
            let y = parse_field(2)?.parse()?;
            let dw = parse_field(3)?.parse()?;

            let entry = BitMapEntry {
                fxy: FXY::new(f, x, y),
                depth: dw,
            };
            entries.push(entry);
        }
        Ok(entries)
    }
}
