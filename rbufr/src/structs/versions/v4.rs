use crate::errors::Result;
use crate::structs::{tools::parse_descriptors, versions::MessageVersion};
use nom::{
    IResult,
    bytes::complete::{tag, take},
    error::{Error, ErrorKind},
    number::complete::{be_u8, be_u16, be_u24},
};

use super::skip1;

#[derive(Clone)]
pub struct BUFRMessageV4 {
    pub section0: Section0,
    pub section1: Section1,
    pub section2: Option<Section2>,
    pub section3: Section3,
    pub section4: Section4,
}

impl MessageVersion for BUFRMessageV4 {
    fn parse(input: &[u8]) -> crate::errors::Result<Self> {
        let (input, section0) = parse_section0(input)?;
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

        Ok(BUFRMessageV4 {
            section0,
            section1,
            section2,
            section3,
            section4,
        })
    }

    fn description(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BUFR Message V4:\n")?;
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

    fn descriptors(&self) -> Result<Vec<genlib::FXY>> {
        parse_descriptors(&self.section3.data)
    }

    fn data_block(&self) -> Result<&[u8]> {
        Ok(&self.section4.data)
    }
}

#[derive(Clone)]
pub struct Section0 {
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

#[derive(Clone, Debug)]
pub struct Section1 {
    pub length: usize,                      // octet 1-3
    pub master_table: u8,                   // octet 4
    pub centre: u16,                        // octet 5-6
    pub subcentre: u16,                     // octet 7-8
    pub update_sequence_number: u8,         // octet 9
    pub optional_section_present: bool,     // octet 10 bit1
    pub data_category: u8,                  // octet 11
    pub international_data_subcategory: u8, // octet 12
    pub local_subcategory: u8,              // octet 13
    pub master_table_version: u8,           // octet 14
    pub local_table_version: u8,            // octet 15
    pub year: u16,                          // octet 16-17 (4 digits)
    pub month: u8,                          // octet 18
    pub day: u8,                            // octet 19
    pub hour: u8,                           // octet 20
    pub minute: u8,                         // octet 21
    pub second: u8,                         // octet 22
    pub local_use: Vec<u8>,                 // octet 23-
}

fn parse_section1(input: &[u8]) -> IResult<&[u8], Section1> {
    let (input, length_u24) = be_u24(input)?;
    let length = length_u24 as usize;

    const FIXED_LEN: usize = 22;
    if length < FIXED_LEN {
        return Err(nom::Err::Error(Error::new(input, ErrorKind::LengthValue)));
    }

    let (input, master_table) = be_u8(input)?;
    let (input, centre) = be_u16(input)?;
    let (input, subcentre) = be_u16(input)?;
    let (input, update_sequence_number) = be_u8(input)?;

    let (input, flags) = be_u8(input)?;
    let optional_section_present = (flags & 0x80) != 0;

    let (input, data_category) = be_u8(input)?;
    let (input, international_data_subcategory) = be_u8(input)?;
    let (input, local_subcategory) = be_u8(input)?;
    let (input, master_table_version) = be_u8(input)?;
    let (input, local_table_version) = be_u8(input)?;

    let (input, year) = be_u16(input)?;
    let (input, month) = be_u8(input)?;
    let (input, day) = be_u8(input)?;
    let (input, hour) = be_u8(input)?;
    let (input, minute) = be_u8(input)?;
    let (input, second) = be_u8(input)?;

    let local_len = length - FIXED_LEN;
    let (input, local_bytes) = take(local_len)(input)?;

    Ok((
        input,
        Section1 {
            length,
            master_table,
            centre,
            subcentre,
            update_sequence_number,
            optional_section_present,
            data_category,
            international_data_subcategory,
            local_subcategory,
            master_table_version,
            local_table_version,
            year,
            month,
            day,
            hour,
            minute,
            second,
            local_use: local_bytes.to_vec(),
        },
    ))
}

impl std::fmt::Display for Section1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Section 1:")?;
        writeln!(f, "  Length: {} bytes", self.length)?;
        writeln!(f)?;
        writeln!(f, "  Organization:")?;
        writeln!(
            f,
            "    Centre:              {:<5} (0x{:04X})",
            self.centre, self.centre
        )?;
        writeln!(
            f,
            "    Sub-centre:          {:<5} (0x{:04X})",
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
        writeln!(
            f,
            "    International Sub:   {}",
            self.international_data_subcategory
        )?;
        writeln!(f, "    Local Sub:           {}", self.local_subcategory)?;
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
            "    DateTime:            {:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )?;
        writeln!(f)?;
        writeln!(f, "  Optional Data:")?;
        writeln!(
            f,
            "    Section 2 Present:   {}",
            if self.optional_section_present {
                "Yes"
            } else {
                "No"
            }
        )?;
        write!(f, "    Local Use Data:      {} bytes", self.local_use.len())
    }
}

#[derive(Clone)]
pub struct Section2 {
    pub length: usize,
    pub data: Vec<u8>,
}

fn parse_section2(input: &[u8]) -> IResult<&[u8], Section2> {
    let (input, length) = be_u24(input)?;
    let (input, _) = skip1(input)?;
    let (input, data) = take(length - 4)(input)?;
    Ok((
        input,
        Section2 {
            length: length as usize,
            data: data.to_vec(),
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

pub struct Section5;

fn parse_section5(input: &[u8]) -> IResult<&[u8], Section5> {
    let (input, _) = tag("7777")(input)?;
    Ok((input, Section5 {}))
}

#[derive(Clone)]
pub struct BUFRMessage {
    pub section0: Section0,
    pub section1: Section1,
    pub section2: Option<Section2>,
    pub section3: Section3,
    pub section4: Section4,
}

impl BUFRMessage {
    pub fn parse(input: &[u8]) -> IResult<&[u8], BUFRMessage> {
        let (input, section0) = parse_section0(input)?;
        let (input, section1) = parse_section1(input)?;
        let (input, section2) = if section1.optional_section_present {
            let (input, sec2) = parse_section2(input)?;
            (input, Some(sec2))
        } else {
            (input, None)
        };
        let (input, section3) = parse_section3(input)?;
        let (input, section4) = parse_section4(input)?;
        let (input, _section5) = parse_section5(input)?;

        Ok((
            input,
            BUFRMessage {
                section0,
                section1,
                section2,
                section3,
                section4,
            },
        ))
    }
}
