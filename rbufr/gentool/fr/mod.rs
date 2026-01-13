use csv::{ReaderBuilder, StringRecord};
use librbufr::core::{
    TableConverter,
    tables::{TableEntryFull, TableTypeTrait},
};
pub mod btable;
pub mod dtable;

pub type FRDTableLoader = TableLoader<dtable::FRDTableLoader>;
pub type FRBTableLoader = TableLoader<btable::BTableLoader>;

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
        let path = path.as_ref();
        let mut entries = vec![];
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .delimiter(b';')
            .flexible(true)
            .from_path(path)?;

        let mut line_num = 1;
        for result in rdr.records() {
            line_num += 1;
            match result {
                Ok(record) => match loader.process_entry(record) {
                    Ok(Some(processed_entry)) => {
                        entries.push(processed_entry);
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Skipping line {} in {}: {}",
                            line_num,
                            path.display(),
                            e
                        );
                    }

                    _ => {}
                },
                Err(e) => {
                    eprintln!(
                        "Warning: Skipping line {} in {}: {}",
                        line_num,
                        path.display(),
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
    type TableType: TableTypeTrait;

    fn process_entry(&mut self, raw: StringRecord) -> anyhow::Result<Option<Self::Output>>;
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
