pub mod config;
pub mod fr;
#[cfg(feature = "opera")]
pub mod opera;
pub mod pattern;
pub mod prelude;
pub mod tables;
mod utils;
pub mod wmo;
use anyhow::Context;
use memmap2::Mmap;
use ph::fmph::GOFunction;
use rkyv::api::high::HighValidator;
use rkyv::bytecheck::CheckBytes;
use rkyv::rancor::Error;
use rkyv::{Archive, Deserialize, Serialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use std::borrow::Borrow;
use std::fmt::Debug;
use std::io::{Cursor, Write};
use std::path::Path;

use crate::tables::{TableEntryFull, TableTypeTrait};

pub trait TableConverter {
    type OutputEntry: TableEntryFull;
    type TableType: TableTypeTrait;
    fn convert<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<Vec<Self::OutputEntry>>;

    fn table_type(&self) -> crate::TableType {
        Self::TableType::TABLE_TYPE
    }
}

struct BufrTableMph<T: TableEntryFull> {
    mphf: GOFunction,
    mmap: Mmap,
    _marker: std::marker::PhantomData<T>,
}

#[derive(Archive, Deserialize, Serialize, PartialEq)]
#[rkyv(compare(PartialEq))]
struct BUFRTF<T>
where
    T: TableEntryFull,
{
    pub function_header: Vec<u8>,
    pub entries: Vec<T>,
}

impl<T> BUFRTF<T>
where
    T: TableEntryFull,
{
    fn new(entries: Vec<T>) -> std::io::Result<Self> {
        let keys: Vec<FXY> = entries.iter().map(|e| e.fxy()).collect();
        let mphf = GOFunction::from_slice(&keys);
        let mut sorted_entries: Vec<(usize, T)> = entries
            .into_iter()
            .map(|e| (mphf.get(&(e.fxy())).unwrap() as usize, e))
            .collect();
        sorted_entries.sort_by_key(|(hash, _)| *hash);

        let mut mphf_bytes = Vec::new();
        mphf.write(&mut mphf_bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", e)))?;

        Ok(Self {
            function_header: mphf_bytes,
            entries: sorted_entries.into_iter().map(|(_, e)| e).collect(),
        })
    }

    fn write_to_disk<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        let mut file = std::fs::File::create(path)?;
        let bytes = rkyv::to_bytes::<Error>(self)?;
        file.write_all(&bytes)?;
        Ok(())
    }
}

impl<T: TableEntryFull> BufrTableMph<T>
where
    <T as Archive>::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>,
{
    fn bufrtbl_path<P: AsRef<Path>>(path: P) -> std::path::PathBuf {
        let mut path = path.as_ref().to_path_buf();
        path.set_extension("bufrtbl");
        path
    }

    fn build<P: AsRef<Path>>(entries: Vec<T>, output_path: P) -> anyhow::Result<Self> {
        let output_path = Self::bufrtbl_path(output_path);
        let bufrtf = BUFRTF::new(entries)?;
        bufrtf.write_to_disk(&output_path)?;

        Self::load(output_path)
    }

    fn load<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = Self::bufrtbl_path(path);

        let merged_file = std::fs::File::open(&path)?;
        let mmap = unsafe { Mmap::map(&merged_file)? };

        let archived = rkyv::access::<ArchivedBUFRTF<T>, Error>(&mmap)?;
        let function_reader = &archived.function_header[..];

        let mut cursor = Cursor::new(function_reader);

        Ok(Self {
            mphf: GOFunction::read(&mut cursor)?,
            mmap,
            _marker: std::marker::PhantomData,
        })
    }

    /// 获取拥有的版本
    fn get<K: BUFRKey>(&self, fxy: &K) -> Option<&<T as Archive>::Archived> {
        let hash = self.mphf.get(&fxy)? as usize;
        self.archived().ok()?.entries.get(hash)
    }

    fn archived(&self) -> anyhow::Result<&ArchivedBUFRTF<T>> {
        let archived = rkyv::access::<ArchivedBUFRTF<T>, Error>(&self.mmap)?;
        Ok(archived)
    }

    /// 获取所有条目
    fn get_all(&self) -> Vec<&<T as Archive>::Archived> {
        if let Ok(archived) = self.archived() {
            let mut result = vec![];
            archived.entries.iter().for_each(|entry| {
                result.push(entry);
            });
            result
        } else {
            vec![]
        }
    }
}

#[derive(
    Archive,
    SerdeSerialize,
    SerdeDeserialize,
    rkyv::Serialize,
    rkyv::Deserialize,
    Debug,
    Eq,
    Clone,
    Copy,
    std::hash::Hash,
)]
#[rkyv(
    // compare(PartialEq),
    derive(Debug, Clone, Copy, std::hash::Hash, Eq)
)]
pub struct FXY {
    pub f: i32,
    pub x: i32,
    pub y: i32,
}

impl FXY {
    pub fn new(f: i32, x: i32, y: i32) -> Self {
        FXY { f, x, y }
    }
    pub fn from_str(fxy_str: &str) -> anyhow::Result<Self> {
        // let bytes = fxy_str.as_bytes();

        if fxy_str.len() != 6 {
            return Err(anyhow::anyhow!("Invalid FXY string length: {}", fxy_str));
        }

        let f = fxy_str[0..2]
            .parse::<i32>()
            .with_context(|| format!("Failed to parse F from FXY: {}", fxy_str))?;

        let x = fxy_str[2..4]
            .parse::<i32>()
            .with_context(|| format!("Failed to parse X from FXY: {}", fxy_str))?;
        let y = fxy_str[4..6]
            .parse::<i32>()
            .with_context(|| format!("Failed to parse Y from FXY: {}", fxy_str))?;

        Ok(FXY { f, x, y })
    }

    /// Convert FXY to u32 for use as hash key
    /// Format: F (2 bits) | X (6 bits) | Y (8 bits) = 16 bits total
    pub fn to_u32(&self) -> u32 {
        ((self.f as u32) << 14) | ((self.x as u32) << 8) | (self.y as u32)
    }
}

pub struct BUFRTableMPH<T: TableTypeTrait> {
    inner: BufrTableMph<T::EntryType>,
}

impl<T: TableTypeTrait> BUFRTableMPH<T>
where
    <T::EntryType as Archive>::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>,
{
    pub fn build_from_csv<P: AsRef<Path>, L: TableConverter>(
        loader: L,
        path: P,
        output_path: P,
    ) -> anyhow::Result<Self>
    where
        L: TableConverter<OutputEntry = T::EntryType>,
        L: TableConverter<TableType = T>,
        <T::EntryType as Archive>::Archived: for<'a> CheckBytes<HighValidator<'a, Error>>,
    {
        let entries = loader.convert(path)?;
        let bhm = BufrTableMph::<T::EntryType>::build(entries, output_path)?;

        Ok(BUFRTableMPH { inner: bhm })
    }

    pub fn get_all_entries(&self) -> Vec<&<T::EntryType as Archive>::Archived> {
        self.inner.get_all()
    }

    pub fn load_from_disk<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let bhm = BufrTableMph::<T::EntryType>::load(path)?;
        Ok(BUFRTableMPH { inner: bhm })
    }

    pub fn lookup<K: BUFRKey>(&self, fxy: &K) -> Option<&<T::EntryType as Archive>::Archived> {
        self.inner.get(fxy)
    }
}

pub trait BUFRKey: Debug + Eq + std::hash::Hash + PartialEq<FXY> + PartialEq<ArchivedFXY> {
    fn f(&self) -> i32;
    fn x(&self) -> i32;
    fn y(&self) -> i32;
}

impl BUFRKey for FXY {
    fn f(&self) -> i32 {
        self.f
    }
    fn x(&self) -> i32 {
        self.x
    }
    fn y(&self) -> i32 {
        self.y
    }
}
impl BUFRKey for ArchivedFXY {
    fn f(&self) -> i32 {
        self.f.to_native()
    }
    fn x(&self) -> i32 {
        self.x.to_native()
    }
    fn y(&self) -> i32 {
        self.y.to_native()
    }
}

impl<K: BUFRKey> PartialEq<K> for FXY {
    fn eq(&self, other: &K) -> bool {
        self.f == other.f() && self.x == other.x() && self.y == other.y()
    }
}

impl<K: BUFRKey> PartialEq<K> for ArchivedFXY {
    fn eq(&self, other: &K) -> bool {
        self.f.to_native() == other.f()
            && self.x.to_native() == other.x()
            && self.y.to_native() == other.y()
    }
}

// impl Borrow<FXY> for ArchivedFXY {
//     fn borrow(&self) -> &FXY {
//         // SAFETY: ArchivedFXY has the same memory layout as FXY
//         unsafe { &*(self as *const ArchivedFXY as *const FXY) }
//     }
// }

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableType {
    B,
    D,
    BitMap,
}

#[cfg(test)]
mod test {
    use crate::{
        BUFRTableMPH,
        BufrTableMph,
        FXY,
        // wmo::{TableLoader, btable::BTableCsvLoader},
        fr::{TableLoader, btable::BTableLoader as BTableCsvLoader},
        prelude::{BUFRTableB, BUFRTableD},
    };

    #[test]
    fn test() {
        let table_loader = TableLoader::<BTableCsvLoader>::default();
        BUFRTableB::build_from_csv(
            table_loader,
            // "/Users/xiang.li1/projects/rbufr/BUFR4/BUFRCREX_TableB_en_42.csv",
            "/Users/xiang.li1/Downloads/tables 2/bufrtabb_16.csv",
            "./test.bufrtbl",
        )
        .unwrap();
    }

    #[test]
    fn load() {
        let table = BUFRTableD::load_from_disk(
            "/Users/xiang.li1/projects/rbufr/rbufr/tables/master/BUFR_TableD_16.bufrtbl",
        )
        .unwrap();

        let x = table.lookup(&FXY::new(3, 21, 11)).unwrap();

        println!("{:#?}", x);
    }
}
