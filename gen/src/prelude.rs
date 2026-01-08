use crate::tables::BTable;
use crate::tables::DTable;
pub use crate::wmo;
pub type BUFRTableD = crate::BUFRTableMPH<DTable>;
pub type BUFRTableB = crate::BUFRTableMPH<BTable>;
pub type BUFRTableBitMap = crate::BUFRTableMPH<crate::tables::BitMap>;
pub use crate::BUFRTableMPH;
pub use crate::FXY;
pub use crate::TableType;
