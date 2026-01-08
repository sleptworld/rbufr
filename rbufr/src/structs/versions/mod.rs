pub mod v2;
pub mod v4;
use std::collections::VecDeque;

pub(super) use super::{skip, skip1, skip2};
use crate::errors::{Error, Result};
use genlib::FXY;
use nom::{
    IResult,
    bytes::complete::{tag, take},
    number::complete::{be_u8, be_u16, be_u24},
};

macro_rules! message {
    ($(($version:ident, $t: ty, $v: expr)),+$(,)?) => {
        #[derive(Clone)]
        pub enum BUFRMessage {
            $(
                $version($t),
            )+
        }

        impl MessageVersion for BUFRMessage {
            fn parse(input: &[u8]) -> Result< Self> {
                let (_, section0) = parse_section0(input)?;
                match section0.version {
                    $(
                        x if x == $v => {
                            let msg = <$t as MessageVersion>::parse(input)?;
                            Ok(BUFRMessage::$version(msg))
                        }
                    )+
                    _ => Err(Error::UnsupportedVersion(section0.version)),
                }
            }

            fn description(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.description(f),
                    )+
                }
            }

            fn table_info(&self) -> TableInfo {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.table_info(),
                    )+
                }
            }

            fn subcenter_id(&self) -> u16 {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.subcenter_id(),
                    )+
                }
            }

            fn center_id(&self) -> u16 {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.center_id(),
                    )+
                }
            }

            fn master_table_version(&self) -> u8 {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.master_table_version(),
                    )+
                }
            }
            fn local_table_version(&self) -> u8 {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.local_table_version(),
                    )+
                }
            }

            fn subsets_count(&self) -> u16 {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.subsets_count(),
                    )+
                }
            }

            fn ndescs(&self) -> usize {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.ndescs(),
                    )+
                }
            }

            fn descriptors(&self) -> Result<Vec<FXY>> {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.descriptors(),
                    )+
                }
            }

            fn data_block(&self) -> Result<&[u8]> {
                match self {
                    $(
                        BUFRMessage::$version(msg) => msg.data_block(),
                    )+
                }
            }
        }
    };
}

message!((V2, v2::BUFRMessageV2, 2), (V4, v4::BUFRMessageV4, 4));

impl BUFRMessage {
    pub fn version(&self) -> u8 {
        match self {
            BUFRMessage::V2(_) => 2,
            BUFRMessage::V4(_) => 4,
        }
    }
}

impl std::fmt::Display for BUFRMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BUFRMessage::V2(msg) => msg.description(f),
            BUFRMessage::V4(msg) => msg.description(f),
        }
    }
}

pub trait MessageVersion: Sized {
    fn parse(input: &[u8]) -> Result<Self>;

    fn description(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;

    fn table_info(&self) -> TableInfo;

    fn subcenter_id(&self) -> u16 {
        self.table_info().subcenter_id
    }

    fn center_id(&self) -> u16 {
        self.table_info().center_id
    }

    fn master_table_version(&self) -> u8 {
        self.table_info().master_table_version
    }

    fn local_table_version(&self) -> u8 {
        self.table_info().local_table_version
    }

    fn subsets_count(&self) -> u16;

    fn ndescs(&self) -> usize;

    fn descriptors(&self) -> Result<Vec<FXY>>;

    fn data_block(&self) -> Result<&[u8]>;
}

pub struct TableInfo {
    pub master_table_version: u8,
    pub local_table_version: u8,
    pub center_id: u16,
    pub subcenter_id: u16,
}

#[derive(Clone)]
struct Section0 {
    pub total_length: u32,
    pub version: u8,
}

fn parse_section0(input: &[u8]) -> IResult<&[u8], Section0> {
    let (input, _) = tag("BUFR")(input)?;
    let (input, total_length) = be_u24(input)?;
    let (input, edition) = be_u8(input)?;
    Ok((
        input,
        Section0 {
            total_length,
            version: edition,
        },
    ))
}
