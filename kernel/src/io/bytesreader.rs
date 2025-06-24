pub struct BytesReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> BytesReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub fn read_u32_be(&mut self) -> u32 {
        let mut bytes = [0; 4];
        let src = &self.bytes[self.pos..(self.pos + 4)];
        bytes.copy_from_slice(src);
        self.pos += 4;
        u32::from_be_bytes(bytes)
    }

    pub fn peek_u32_be(&mut self) -> u32 {
        let mut bytes = [0; 4];
        let src = &self.bytes[self.pos..(self.pos + 4)];
        bytes.copy_from_slice(src);
        u32::from_be_bytes(bytes)
    }

    pub fn read_u64_be(&mut self) -> u64 {
        let mut bytes = [0; 8];
        let src = &self.bytes[self.pos..(self.pos + 8)];
        bytes.copy_from_slice(src);
        self.pos += 8;
        u64::from_be_bytes(bytes)
    }

    pub fn read_bytes(&mut self, count: usize) -> &'a [u8] {
        let bytes = &self.bytes[self.pos..(self.pos + count)];
        self.pos += count;
        bytes
    }

    pub fn skip(&mut self, count: usize) {
        self.pos += count;
    }

    pub fn align_to(&mut self, align: usize) {
        let dif = self.pos % align;
        if dif != 0 {
            self.pos = (self.pos - dif) + align;
        }
    }

    pub fn null_term_str(&mut self) -> &'a str {
        read_null_term_str(self.bytes, &mut self.pos)
    }

    pub fn pos(&self) -> usize {
        self.pos
    }
}

pub fn read_null_term_str<'a>(bytes: &'a [u8], pos: &mut usize) -> &'a str {
    let mut len = 0;
    while 0 != bytes[*pos + len] {
        len += 1;
    }

    let slice = &bytes[*pos..(*pos + len)];
    *pos += len + 1;
    str::from_utf8(slice).expect("invalid string")
}
