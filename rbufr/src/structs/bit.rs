use nom::IResult;
use nom::bits::complete::take;
use std::ops::{AddAssign, Shl, Shr};
pub(super) type BitInput<'a> = (&'a [u8], usize);

pub(super) fn parse_arbitrary_bits<
    T: From<u8> + AddAssign + Shl<usize, Output = T> + Shr<usize, Output = T>,
>(
    input: BitInput,
    count: usize,
) -> IResult<BitInput, T> {
    take(count)(input)
}
