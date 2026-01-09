use nom::{IResult, bytes::complete::take};
pub(super) mod bit;
pub(super) mod tools;
pub mod versions;
#[cfg(feature = "opera")]
pub const GENCENTER: u16 = 247;

#[inline]
pub fn skip(n: usize) -> impl Fn(&[u8]) -> IResult<&[u8], ()> {
    move |input: &[u8]| {
        let (input, _) = take(n)(input)?;
        Ok((input, ()))
    }
}

#[inline]
pub fn skip1(input: &[u8]) -> IResult<&[u8], ()> {
    skip(1)(input)
}

#[inline]
pub fn skip2(input: &[u8]) -> IResult<&[u8], ()> {
    skip(2)(input)
}
