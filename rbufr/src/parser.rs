use crate::errors::Result;
use crate::structs::versions::BUFRMessage;
use crate::{block::BUFRFile, structs::versions::MessageVersion};
use flate2::read::GzDecoder;
use std::{
    fs::File,
    io::{BufReader, Cursor, Read, Seek, SeekFrom},
    path::Path,
};

const BUFR_PATTERN: &[u8] = b"BUFR";
const BUFFER_SIZE: usize = 8192;

pub fn parse<P: AsRef<Path>>(path: P) -> Result<BUFRFile> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut magic_bytes = [0u8; 2];
    reader.read_exact(&mut magic_bytes)?;
    reader.seek(SeekFrom::Start(0))?;
    if magic_bytes == [0x1F, 0x8B] {
        let mut gz_decoder = GzDecoder::new(reader);
        let mut bytes = vec![];
        gz_decoder.read_to_end(&mut bytes)?;

        parse_inner(&mut Cursor::new(bytes))
    } else {
        reader.seek(SeekFrom::Start(0))?;
        parse_inner(&mut reader)
    }
}

fn find_bufr_offsets<R: Read + Seek>(reader: &mut R) -> Result<Vec<u64>> {
    let mut offsets = Vec::new();
    let mut buffer = vec![0u8; BUFFER_SIZE];
    let mut file_offset = 0u64;
    let mut overlap = vec![0u8; BUFR_PATTERN.len() - 1];
    let mut overlap_len = 0;

    reader.seek(SeekFrom::Start(0))?;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let mut search_buffer = Vec::with_capacity(overlap_len + bytes_read);
        search_buffer.extend_from_slice(&overlap[..overlap_len]);
        search_buffer.extend_from_slice(&buffer[..bytes_read]);

        for i in 0..search_buffer.len().saturating_sub(BUFR_PATTERN.len() - 1) {
            if search_buffer.len() >= i + BUFR_PATTERN.len()
                && &search_buffer[i..i + BUFR_PATTERN.len()] == BUFR_PATTERN
            {
                let actual_offset = file_offset - overlap_len as u64 + i as u64;
                offsets.push(actual_offset);
            }
        }

        if bytes_read >= BUFR_PATTERN.len() - 1 {
            overlap_len = BUFR_PATTERN.len() - 1;
            overlap[..overlap_len].copy_from_slice(&buffer[bytes_read - overlap_len..bytes_read]);
        } else {
            overlap_len = bytes_read;
            overlap[..overlap_len].copy_from_slice(&buffer[..bytes_read]);
        }

        file_offset += bytes_read as u64;
    }

    Ok(offsets)
}

fn read_message_at_offset<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<Vec<u8>> {
    reader.seek(SeekFrom::Start(offset))?;

    let mut section0_buf = [0u8; 8];
    reader.read_exact(&mut section0_buf)?;

    let total_length = u32::from_be_bytes([0, section0_buf[4], section0_buf[5], section0_buf[6]]);

    let mut message_buf = vec![0u8; total_length as usize];
    reader.seek(SeekFrom::Start(offset))?;
    reader.read_exact(&mut message_buf)?;

    Ok(message_buf)
}

fn parse_inner<R>(buf_reader: &mut R) -> Result<BUFRFile>
where
    R: Read + Seek,
{
    let offsets = find_bufr_offsets(buf_reader)?;
    let mut file_block = BUFRFile::new();

    for offset in offsets {
        match read_message_at_offset(buf_reader, offset) {
            Ok(message_data) => match BUFRMessage::parse(&message_data) {
                Ok(message) => {
                    file_block.push_message(message);
                }
                Err(e) => {
                    eprintln!("Failed to parse BUFR message at offset {}: {:?}", offset, e);
                }
            },
            Err(e) => {
                eprintln!("Failed to read BUFR message at offset {}: {:?}", offset, e);
            }
        }
    }

    Ok(file_block)
}
