pub use crate::core::prelude::{BUFRTableB, BUFRTableD, TableType};
use crate::core::{prelude::*, tables::TableTypeTrait};
use crate::errors::Result;
use std::path::PathBuf;

pub trait TableTrait {
    fn file_path(&self, table_type: TableType) -> PathBuf;
}

#[derive(Debug, Clone, Copy)]
pub struct MasterTable {
    version: u8,
}

impl MasterTable {
    pub fn new(version: u8) -> Self {
        MasterTable { version }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct LocalTable {
    sub_center: Option<u16>,
    version: u8,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub struct BitmapTable {
    center: u16,
    subcenter: u16,
    local_version: u8,
    master_version: u8,
}

impl BitmapTable {
    pub fn new(center: u16, subcenter: u16, local_version: u8, master_version: u8) -> Self {
        BitmapTable {
            center,
            subcenter,
            local_version,
            master_version,
        }
    }
}

impl LocalTable {
    pub fn new(sub_center: Option<u16>, version: u8) -> Self {
        LocalTable {
            sub_center,
            version,
        }
    }
}
impl TableTrait for MasterTable {
    fn file_path(&self, table_type: TableType) -> PathBuf {
        use crate::table_path::get_table_path;

        match table_type {
            TableType::B => {
                let file_name = format!("master/BUFR_TableB_{}.bufrtbl", self.version);
                get_table_path(file_name)
            }
            TableType::D => {
                let file_name = format!("master/BUFR_TableD_{}.bufrtbl", self.version);
                get_table_path(file_name)
            }
            _ => {
                unreachable!("Table type not supported for MasterTable")
            }
        }
    }
}

impl TableTrait for LocalTable {
    fn file_path(&self, table_type: TableType) -> PathBuf {
        use crate::table_path::get_table_path;

        match table_type {
            TableType::B => {
                let sub_center_str = match self.sub_center {
                    Some(sc) => format!("{}", sc),
                    None => "0".to_string(),
                };
                let file_name = format!(
                    "local/BUFR_TableB_{}_{}.bufrtbl",
                    sub_center_str, self.version
                );
                get_table_path(file_name)
            }
            TableType::D => {
                let sub_center_str = match self.sub_center {
                    Some(sc) => format!("{}", sc),
                    None => "0".to_string(),
                };
                let file_name = format!(
                    "local/BUFR_TableD_{}_{}.bufrtbl",
                    sub_center_str, self.version
                );
                get_table_path(file_name)
            }
            _ => {
                unreachable!("Table type not supported for LocalTable")
            }
        }
    }
}

impl TableTrait for BitmapTable {
    fn file_path(&self, table_type: TableType) -> PathBuf {
        use crate::table_path::get_table_path;

        match table_type {
            TableType::BitMap => {
                let file_name = format!("opera/BUFR_Opera_Bitmap_{}.bufrtbl", self.center);
                get_table_path(file_name)
            }
            _ => {
                unreachable!("Table type not supported for BitmapTable")
            }
        }
    }
}

pub struct TableLoader;

impl TableLoader {
    pub fn load_table<T>(&self, table_type: impl TableTrait) -> Result<BUFRTableMPH<T>>
    where
        T: TableTypeTrait,
    {
        let path = table_type.file_path(T::TABLE_TYPE);
        // println!("Loading table from {:?}", path);
        BUFRTableMPH::<T>::load_from_disk(path).map_err(|e| e.into())
    }
}
