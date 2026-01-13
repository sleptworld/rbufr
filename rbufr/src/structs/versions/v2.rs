use crate::core::FXY;
use nom::{
    IResult,
    bytes::complete::{tag, take},
    number::complete::{be_u8, be_u16, be_u24},
};

use crate::errors::Result;
use crate::structs::{tools::parse_descriptors, versions::MessageVersion};

use super::{Section2, parse_section0, parse_section2, skip1};

#[derive(Clone)]
pub struct BUFRMessageV2 {
    pub section1: Section1,
    pub section2: Option<Section2>,
    pub section3: Section3,
    pub section4: Section4,
}

impl MessageVersion for BUFRMessageV2 {
    fn parse(input: &[u8]) -> crate::errors::Result<Self> {
        let (input, _) = parse_section0(input)?;
        let (input, section1) = parse_section1(input)?;
        let (input, section2) = if section1.optional_section_present {
            let (input, sec2) = parse_section2(input)?;
            (input, Some(sec2))
        } else {
            (input, None)
        };
        let (input, section3) = parse_section3(input)?;
        let (input, section4) = parse_section4(input)?;
        let (_input, _section5) = parse_section5(input)?;

        Ok(BUFRMessageV2 {
            section1,
            section2,
            section3,
            section4,
        })
    }

    fn description(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BUFR Message V2:\n")?;
        write!(f, "{}\n", self.section1)?;
        Ok(())
    }
    fn table_info(&self) -> super::TableInfo {
        super::TableInfo {
            master_table_version: self.section1.master_table_version,
            local_table_version: self.section1.local_table_version,
            center_id: self.section1.centre as u16,
            subcenter_id: self.section1.subcentre as u16,
        }
    }
    fn subsets_count(&self) -> u16 {
        self.section3.number_of_subsets
    }

    fn ndescs(&self) -> usize {
        self.section3.data.len() / 2
    }

    fn descriptors(&self) -> Result<Vec<FXY>> {
        parse_descriptors(&self.section3.data)
    }

    fn data_block(&self) -> Result<&[u8]> {
        Ok(&self.section4.data)
    }
}

#[derive(Clone, Debug)]
pub struct Section1 {
    pub length: usize,
    pub master_table: u8,               // octet 4
    pub subcentre: u8,                  // octet 5
    pub centre: u8,                     // octet 6
    pub update_sequence_number: u8,     // octet 7
    pub optional_section_present: bool, // octet 8 bit1 (MSB)
    pub data_category: u8,              // octet 9
    pub data_subcategory: u8,           // octet 10
    pub master_table_version: u8,       // octet 11
    pub local_table_version: u8,        // octet 12
    pub year: u8,                       // octet 13 (year of century)
    pub month: u8,                      // octet 14
    pub day: u8,                        // octet 15
    pub hour: u8,                       // octet 16
    pub minute: u8,                     // octet 17
                                        // octet 18- local use: 你可以选择保存或直接跳过
}

fn parse_section1(input: &[u8]) -> IResult<&[u8], Section1> {
    let (input, length) = be_u24(input)?;
    let length = length as usize;

    const FIXED_LEN: usize = 17;
    if length < FIXED_LEN {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::LengthValue,
        )));
    }

    let (input, master_table) = be_u8(input)?;
    let (input, subcentre) = be_u8(input)?;
    let (input, centre) = be_u8(input)?;
    let (input, update_sequence_number) = be_u8(input)?;
    let (input, optional_section_flag) = be_u8(input)?;
    let optional_section_present = (optional_section_flag & 0x80) != 0;

    let (input, data_category) = be_u8(input)?;
    let (input, data_subcategory) = be_u8(input)?;
    let (input, master_table_version) = be_u8(input)?;
    let (input, local_table_version) = be_u8(input)?;
    let (input, year) = be_u8(input)?;
    let (input, month) = be_u8(input)?;
    let (input, day) = be_u8(input)?;
    let (input, hour) = be_u8(input)?;
    let (input, minute) = be_u8(input)?;

    // 剩余 local-use
    let local_len = length - FIXED_LEN;
    let (input, _) = nom::bytes::complete::take(local_len)(input)?;

    Ok((
        input,
        Section1 {
            length,
            master_table,
            subcentre,
            centre,
            update_sequence_number,
            optional_section_present,
            data_category,
            data_subcategory,
            master_table_version,
            local_table_version,
            year,
            month,
            day,
            hour,
            minute,
        },
    ))
}

#[derive(Clone)]
pub struct Section3 {
    pub length: usize,
    pub number_of_subsets: u16,
    pub is_observation: bool,
    pub is_compressed: bool,
    pub data: Vec<u8>,
}

fn parse_section3(input: &[u8]) -> IResult<&[u8], Section3> {
    let (input, length) = be_u24(input)?;
    let (input, _) = skip1(input)?;
    let (input, number_of_subsets) = be_u16(input)?;
    let (input, flags) = be_u8(input)?;
    let is_observation = (flags & 0b1000_0000) != 0;
    let is_compressed = (flags & 0b0100_0000) != 0;
    let (input, data) = take(length - 7)(input)?;
    Ok((
        input,
        Section3 {
            length: length as usize,
            number_of_subsets,
            is_observation,
            is_compressed,
            data: data.to_vec(),
        },
    ))
}

#[derive(Clone)]
pub struct Section4 {
    pub length: usize,
    pub data: Vec<u8>,
}

fn parse_section4(input: &[u8]) -> IResult<&[u8], Section4> {
    let (input, length) = be_u24(input)?;
    let (input, _) = skip1(input)?;
    let (input, data) = take(length - 4)(input)?;
    Ok((
        input,
        Section4 {
            length: length as usize,
            data: data.to_vec(),
        },
    ))
}

impl std::fmt::Display for Section1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Section 1 (BUFR v2):")?;
        writeln!(f, "  Length: {} bytes", self.length)?;
        writeln!(f)?;
        writeln!(f, "  Organization:")?;
        writeln!(
            f,
            "    Centre:              {:<5} (0x{:02X})",
            self.centre, self.centre
        )?;
        writeln!(
            f,
            "    Sub-centre:          {:<5} (0x{:02X})",
            self.subcentre, self.subcentre
        )?;
        writeln!(
            f,
            "    Update Sequence:     {}",
            self.update_sequence_number
        )?;
        writeln!(f)?;
        writeln!(f, "  Data Classification:")?;
        writeln!(f, "    Category:            {}", self.data_category)?;
        writeln!(f, "    Sub-category:        {}", self.data_subcategory)?;
        writeln!(f)?;
        writeln!(f, "  Table Versions:")?;
        writeln!(
            f,
            "    Master Table:        {} (v{})",
            self.master_table, self.master_table_version
        )?;
        writeln!(f, "    Local Table:         v{}", self.local_table_version)?;
        writeln!(f)?;
        writeln!(f, "  Observation Time:")?;
        writeln!(
            f,
            "    DateTime:            19{:02}-{:02}-{:02} {:02}:{:02}:00 UTC",
            self.year, self.month, self.day, self.hour, self.minute
        )?;
        writeln!(f)?;
        writeln!(f, "  Optional Data:")?;
        write!(
            f,
            "    Section 2 Present:   {}",
            if self.optional_section_present {
                "Yes"
            } else {
                "No"
            }
        )
    }
}

pub struct Section5;

fn parse_section5(input: &[u8]) -> IResult<&[u8], Section5> {
    let (input, _) = tag("7777")(input)?;
    Ok((input, Section5 {}))
}
