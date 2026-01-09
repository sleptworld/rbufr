use crate::decoder::Value;

pub struct OperaBitmapParser {
    values: Vec<Value>,
    #[allow(unused)]
    dw: u8,
}

impl OperaBitmapParser {
    pub fn new(dw: u8) -> Self {
        OperaBitmapParser { values: vec![], dw }
    }

    pub fn values(&mut self) -> &mut Vec<Value> {
        &mut self.values
    }
}
