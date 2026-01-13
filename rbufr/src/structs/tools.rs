use crate::core::FXY;
use crate::errors::{Error, Result};
use crate::structs::bit::{BitInput, parse_arbitrary_bits};
use nom::IResult;

pub(super) fn parse_descriptors(input: &[u8]) -> Result<Vec<FXY>> {
    parse_descriptors_inner(input)
        .map(|(_, v)| v)
        .map_err(|_| Error::ParseError(format!("Can't parse descriptors from section3")))
}

fn parse_descriptors_inner(mut input: &[u8]) -> IResult<BitInput<'_>, Vec<FXY>> {
    let mut results = Vec::new();
    while input.len() > 1 {
        let ((finput, _), fxy) = take_fxy((input, 0))?;
        results.push(fxy);
        input = finput;
    }

    Ok(((input, 0), results))
}

fn take_fxy(bit_input: BitInput) -> IResult<BitInput, FXY> {
    let (bit_input, f) = parse_arbitrary_bits(bit_input, 2)?;
    let (bit_input, x) = parse_arbitrary_bits(bit_input, 6)?;
    let (bit_input, y) = parse_arbitrary_bits(bit_input, 8)?;

    Ok((bit_input, FXY::new(f, x, y)))
}
