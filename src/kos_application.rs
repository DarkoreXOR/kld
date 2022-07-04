use crate::writer::{Writer, Operation, BufferRegion};

pub struct KosApplication<'a> {
    writer: Writer<'a>,
}

struct Header {
    code_begin_offset: BufferRegion,
    code_end_offset: BufferRegion,
    memory_offset: BufferRegion,
    stack_offset: BufferRegion,
}

impl<'a> KosApplication<'a> {
    pub fn new(filename: &'a str) -> Self {
        Self {
            writer: Writer::new(filename)
        }
    }

    pub fn get_offsets(&mut self, code_size: usize, data_size: usize) -> (usize, usize) {
        let code = vec![0; code_size];
        let data = vec![0; data_size];
        self.build(code, data, false)
    }

    pub fn build(&mut self, code: Vec<u8>, data: Vec<u8>, create_file: bool) -> (usize, usize) {
        
        self.writer.clear();

        let header = self.write_header();
        
        let writer = &mut self.writer;     
        
        // emit code
        writer.append_padding(4, None);

        let code_begin_offset = writer.offset();

        writer.insert_buffer(Operation::Append, &code);

        let code_end_offset = writer.offset();

        // fix header for code

        writer.insert_u32(
            Operation::Update(header.code_begin_offset),
            code_begin_offset as u32
        );

        writer.insert_u32(
            Operation::Update(header.code_end_offset),
            code_end_offset as u32
        );

        // emit stack

        writer.append_padding(16, None);
        writer.append_dup(0, 4096); // now = 4KB, 4 MB = 4 * 256 * 4096, 
        writer.append_padding(16, None);

        let stack_offset = writer.offset();

        // fix header for stack

        writer.insert_u32(
            Operation::Update(header.stack_offset),
            stack_offset as u32
        );

        // emit data

        let memory_begin_offset = writer.offset();

        writer.insert_buffer(Operation::Append, &data);

        let memory_end_offset = writer.offset();

        // fix header for data

        writer.insert_u32(
            Operation::Update(header.memory_offset),
            memory_end_offset as u32
        );

        // write to file

        if create_file {
            self.writer.write();
        }

        (code_begin_offset, memory_begin_offset)
    }

    fn write_header(&mut self) -> Header {
        let writer = &mut self.writer;

        writer.insert_string(Operation::Append, "MENUET01"); // identifier
        writer.insert_u32(Operation::Append, 1); // version

        // code start (entry point)
        let code_begin_offset = writer.insert_u32(
            Operation::Append, 
            0
        ); 
 
        // code_end
        let code_end_offset = writer.insert_u32(
            Operation::Append, 
            0
        );

         // data region offset
        let memory_offset = writer.insert_u32(
            Operation::Append, 
            0
        );

        // stack region offset (initial ESP)
        let stack_offset = writer.insert_u32(
            Operation::Append, 
            0
        );

        writer.insert_u32(Operation::Append, 0); // params
        writer.insert_u32(Operation::Append, 0); // icon

        Header {
            code_begin_offset,
            code_end_offset,
            memory_offset,
            stack_offset
        }
    }
}
