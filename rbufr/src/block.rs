use std::ops::Deref;

use genlib::BUFRTableMPH;
#[cfg(feature = "opera")]
use genlib::prelude::BUFRTableBitMap;
use genlib::tables::TableTypeTrait;

use crate::decoder::*;
use crate::errors::Result;
#[cfg(feature = "opera")]
use crate::structs::GENCENTER;
use crate::structs::versions::{BUFRMessage, MessageVersion};
use crate::tables::*;

#[derive(Clone)]
pub struct MessageBlock {
    message: BUFRMessage,
}

impl std::fmt::Display for MessageBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Deref for MessageBlock {
    type Target = BUFRMessage;

    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl MessageBlock {
    pub fn new(message: BUFRMessage) -> Self {
        MessageBlock { message }
    }

    pub(crate) fn load_first_validable_table<E: TableTypeTrait>(
        &self,
        table_version: u8,
    ) -> Result<BUFRTableMPH<E>> {
        (0..=table_version)
            .rev()
            .find_map(|version| {
                TableLoader
                    .load_table(MasterTable::new(version))
                    .ok()
                    .inspect(|_| {
                        if version != table_version {
                            eprintln!("Falling back to Master Table version {}", version);
                        }
                    })
            })
            .ok_or(crate::errors::Error::TableNotFoundEmpty)
    }

    #[cfg(feature = "opera")]
    pub(crate) fn load_opera_bitmap_table(
        &self,
        subcenter: u16,
        center: u16,
        local_version: u8,
        master_version: u8,
    ) -> Result<BUFRTableBitMap> {
        TableLoader.load_table(BitmapTable::new(
            center,
            subcenter,
            local_version,
            master_version,
        ))
    }
}

pub struct BUFRFile {
    messages: Vec<MessageBlock>,
}

impl BUFRFile {
    pub fn new() -> Self {
        BUFRFile {
            messages: Vec::new(),
        }
    }

    pub(crate) fn push_message(&mut self, message: BUFRMessage) {
        self.messages.push(MessageBlock::new(message));
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn message_at(&self, index: usize) -> Option<&MessageBlock> {
        self.messages.get(index)
    }

    pub fn messages(&self) -> &[MessageBlock] {
        &self.messages
    }
}
