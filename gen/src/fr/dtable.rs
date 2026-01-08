use super::EntryLoader;
use crate::{
    FXY,
    tables::{DTable, DTableEntry},
};
use csv::StringRecord;
use std::collections::HashSet;

#[derive(Default)]
pub struct FRDTableLoader {
    current_chain: Option<DTableEntry>,
    seen_keys: HashSet<FXY>,
}

impl EntryLoader for FRDTableLoader {
    type Output = DTableEntry;
    type TableType = DTable;

    fn process_entry(&mut self, raw: StringRecord) -> anyhow::Result<Option<Self::Output>> {
        // Skip empty lines
        if raw.len() < 6 {
            return Ok(None);
        }

        if raw.iter().all(|s| s.trim().is_empty()) {
            return Ok(None);
        }

        let parse_field = |index: usize| {
            raw.get(index)
                .map(|s| {
                    let mut s = s.to_string();
                    s.retain(|c| c.is_alphanumeric());
                    s
                })
                .filter(|c| !c.is_empty())
                .ok_or_else(|| anyhow::anyhow!("Missing field at index {}", index))
        };

        // Check if this is a new sequence (columns 0-2 are not empty) or a continuation line
        let is_new_sequence =
            parse_field(0).is_ok() && parse_field(1).is_ok() && parse_field(2).is_ok();

        if is_new_sequence {
            // Parse the sequence descriptor (columns 0-2)
            let f = parse_field(0)?.parse()?;
            let x = parse_field(1)?.parse()?;
            let y = parse_field(2)?.parse()?;
            let fxy = FXY::new(f, x, y);

            // Check for duplicate key and skip if found
            if self.seen_keys.contains(&fxy) {
                eprintln!(
                    "Warning: Duplicate sequence descriptor {:?} - skipping",
                    fxy
                );
                // Skip duplicate entry - we'll ignore all lines for this sequence
                return Ok(None);
            }

            // Parse the first element in the chain (columns 3-5)
            let f1 = parse_field(3)?.parse()?;
            let x1 = parse_field(4)?.parse()?;
            let y1 = parse_field(5)?.parse()?;
            let fxy1 = FXY::new(f1, x1, y1);

            // If we have a current chain, it's finished - return it
            let finished = self.current_chain.take();

            // Start a new chain
            self.seen_keys.insert(fxy);
            let entry = DTableEntry {
                fxy,
                fxy_chain: vec![fxy1],
                category: None,
                category_of_sequences_en: None,
                title_en: None,
                subtitle_en: None,
                note_en: None,
                note_ids: None,
                status: None,
            };
            self.current_chain = Some(entry);

            return Ok(finished);
        } else {
            // Continuation line - add to current chain
            if self.current_chain.is_none() {
                return Err(anyhow::anyhow!(
                    "Continuation line without a sequence header"
                ));
            }

            // Parse the element in the chain (columns 3-5)
            let f1 = parse_field(3)?.parse()?;
            let x1 = parse_field(4)?.parse()?;
            let y1 = parse_field(5)?.parse()?;
            let fxy1 = FXY::new(f1, x1, y1);

            self.current_chain.as_mut().unwrap().fxy_chain.push(fxy1);

            return Ok(None);
        }
    }

    fn finish(&mut self) -> anyhow::Result<Option<Self::Output>> {
        let result = self.current_chain.take();
        if let Some(ref entry) = result {
            println!(
                "Finishing with sequence: {:?} ({} elements)",
                entry.fxy,
                entry.fxy_chain.len()
            );
        }
        Ok(result)
    }
}
