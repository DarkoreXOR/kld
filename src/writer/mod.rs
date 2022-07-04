use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BufferRegion(usize, usize); // (offset_begin, offset_end)

impl BufferRegion {
    pub fn new(offset: usize, size: usize) -> Self {
        Self(offset, offset + size)
    }

    pub fn range(&self) -> Range<usize> {
        (self.0)..(self.1)
    }
}

/// Insert operation.
pub enum Operation {
    /// Appends data to the end of internal buffer.
    Append,

    /// Updates internal buffer with new data.
    Update(BufferRegion)
}

pub struct Writer<'a> {
    filename: &'a str,
    buffer: Vec<u8>,
}

impl<'a> Writer<'a> {    
    pub fn new(filename: &'a str) -> Self {
        Self {
            filename,
            buffer: Vec::with_capacity(4096), // 4KB
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn append_padding(&mut self, alignment: usize, filler: Option<u8>) {
        while self.buffer.len() % alignment > 0 {
            self.buffer.push(
                filler.unwrap_or(0)
            );
        }
    }

    pub fn insert_buffer(&mut self, operation: Operation, buffer: &[u8]) -> BufferRegion {
        self.insert(operation, buffer)
    }

    pub fn insert_string(&mut self, operation: Operation, value: &str) -> BufferRegion {
        let bytes = value.as_bytes();
        self.insert(operation, bytes)
    }

    pub fn insert_u32(&mut self, operation: Operation, value: u32) -> BufferRegion {
        let bytes = &value.to_le_bytes();
        self.insert(operation, bytes)
    }

    pub fn append_dup(&mut self, filler: u8, count: usize) -> BufferRegion {
        let bytes = vec![filler; count];
        self.append_buffer(&bytes)
    }

    /// Writes internal buffer to file.
    pub fn write(&self) {
        std::fs::write(
            self.filename,
            &self.buffer
        )
        .unwrap(); // TODO: better error handling
    }

    pub fn offset(&self) -> usize {
        self.buffer.len()
    }

    fn insert(&mut self, operation: Operation, buffer: &[u8]) -> BufferRegion {
        match operation {
            Operation::Append => 
                self.append_buffer(buffer),

            Operation::Update(buffer_region) => 
                self.update_buffer(buffer_region, buffer),
        }
    }

    fn append_buffer(&mut self, buffer: &[u8]) -> BufferRegion {
        let offset = self.offset();
        self.buffer.extend(buffer);
        BufferRegion::new(offset, buffer.len())
    }

    fn update_buffer(&mut self, buffer_region: BufferRegion, buffer: &[u8]) -> BufferRegion {
        self.buffer[buffer_region.range()]
            .copy_from_slice(buffer);

        buffer_region
    }
}
