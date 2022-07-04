mod object_file;
pub use object_file::*;

use super::context::Context;

pub trait ParsableFile {
    fn parse(&mut self, buffer: Vec<u8>, context: &mut Context) -> Result<(), ()>;
}

