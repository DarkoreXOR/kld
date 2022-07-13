mod object_file;
pub use object_file::*;

use super::context::Context;

pub trait FileParser {
    fn parse(&mut self, data: &[u8], context: &mut Context) -> Result<(), ()>;
}
