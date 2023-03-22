#[derive(Debug)]
pub struct RespArray {
   pub size: usize,
   pub data: Vec<String>
}

#[derive(Debug)]
pub struct RespSimpleString {
    pub size: usize,
    pub data: String
}

impl RespArray {
    pub fn set_array_size(&mut self, size: usize) {
        self.size = size;
    }

    pub fn add_to_array(&mut self, value: String) {
        self.data.push(value);
    }
}
