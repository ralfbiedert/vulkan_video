//! Operations that can be submitted to a queue (e.g., compute, mem copy, or video decode).
use crate::error::Error;
use crate::queue::CommandBuilder;

mod compute;
mod copyb2b;
mod copyi2b;
mod decodeh264;
mod dummy;
mod fill;

/// Something that can be added to a command buffer (e.g., compute, mem copy, or video decode).
pub trait AddToCommandBuffer {
    fn run_in(&self, builder: &mut CommandBuilder) -> Result<(), Error>;
}

pub use compute::Compute;
pub use copyb2b::CopyBuffer2Buffer;
pub use copyi2b::CopyImage2Buffer;
pub use decodeh264::{DecodeH264, DecodeInfo};
pub use dummy::Dummy;
pub use fill::FillBuffer;
