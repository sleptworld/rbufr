pub(crate) fn fxy_str_to_u32(fxy: &str) -> Option<u32> {
    let bytes = fxy.as_bytes();
    if bytes.len() != 5 || bytes[2] != b'-' {
        return None;
    }

    let class_no = (bytes[0] as char).to_digit(10)? * 10 + (bytes[1] as char).to_digit(10)?;
    let element_no = (bytes[3] as char).to_digit(10)? * 100 + (bytes[4] as char).to_digit(10)? * 10;

    Some(class_no * 1000 + element_no)
}
