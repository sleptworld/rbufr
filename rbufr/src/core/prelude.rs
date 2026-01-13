use super::tables::BTable;
use super::tables::DTable;
pub type BUFRTableD = super::BUFRTableMPH<DTable>;
pub type BUFRTableB = super::BUFRTableMPH<BTable>;
pub type BUFRTableBitMap = super::BUFRTableMPH<super::tables::BitMap>;
pub use super::BUFRTableMPH;
pub use super::FXY;
pub use super::TableType;
