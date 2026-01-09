use librbufr::{decoder::Decoder, parser::parse};

#[test]
fn test_dec() {
    let file = std::fs::read("example/datas/36_2025-12-17T09_00_00.bufr").unwrap();
    let file = parse(&file).unwrap();
    for msg in file.messages() {
        let mut decoder = Decoder::from_message(msg).unwrap();
        let record = decoder.decode(msg).unwrap();

        println!("{}", record);
    }
}
