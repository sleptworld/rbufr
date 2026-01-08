pub mod btable;
pub mod dtable;
use crate::{
    TableConverter,
    tables::{TableEntryFull, TableTypeTrait},
};
pub use btable::BTableCsvLoader as WMOBTableLoader;
use csv::ReaderBuilder;
pub use dtable::DTableCsvLoader as WMODTableLoader;
use std::fmt::Debug;

#[derive(Default)]
pub struct TableLoader<C: EntryLoader> {
    _marker: std::marker::PhantomData<C>,
}

impl<C: EntryLoader> TableLoader<C> {
    pub fn load_table<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        loader: &mut C,
    ) -> anyhow::Result<Vec<C::Output>> {
        let mut entries = vec![];
        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .delimiter(b',')
            .flexible(true) // Allow variable number of fields
            .from_path(path.as_ref())?;

        let mut line_num = 1; // Start at 1 for header
        for result in rdr.deserialize() {
            line_num += 1;
            match result {
                Ok(record) => {
                    let record: C::RawEntry = record;
                    if let Some(processed_entry) = loader.process_entry(record)? {
                        entries.push(processed_entry);
                    }
                }
                Err(e) => {
                    // Log the error but continue processing
                    eprintln!(
                        "Warning: Skipping line {} in {}: {}",
                        line_num,
                        path.as_ref().display(),
                        e
                    );
                }
            }
        }

        if let Some(processed_entry) = loader.finish()? {
            entries.push(processed_entry);
        }
        Ok(entries)
    }
}

pub trait EntryLoader: Default {
    type Output: TableEntryFull;
    type RawEntry: for<'de> serde::Deserialize<'de> + Debug;
    type TableType: TableTypeTrait;

    fn process_entry(&mut self, raw: Self::RawEntry) -> anyhow::Result<Option<Self::Output>>;

    fn finish(&mut self) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }
}

impl<T: EntryLoader> TableConverter for TableLoader<T> {
    type OutputEntry = T::Output;
    type TableType = T::TableType;

    fn convert<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> anyhow::Result<Vec<Self::OutputEntry>> {
        let mut loader = T::default();
        self.load_table(path, &mut loader)
    }
}
