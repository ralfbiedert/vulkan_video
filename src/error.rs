use thiserror::Error as E;

/// Indicates what kind of error occurred.
#[derive(E, Debug)]
pub enum Error {
    #[error("A NUL byte was encountered")]
    Nul(#[from] std::ffi::NulError),

    #[error("CStr too large for static array")]
    CStrTooLargeForStaticArray(#[from] ash::vk::CStrTooLargeForStaticArray),

    #[error("Could not load Vulkan")]
    Loading(#[from] ash::LoadingError),

    #[error("General Vulkan error")]
    Vulkan(#[from] ash::vk::Result),

    #[error("No suitable video device found")]
    NoVideoDevice,

    #[error("No compute pipeline found")]
    NoComputePipeline,

    #[error("No command buffer found")]
    NoCommandBuffer,

    #[error("Vulkan heap not found")]
    HeapNotFound,

    #[error("Vulkan queue not found")]
    QueueNotFound,

    #[error("Image was already bound")]
    ImageAlreadyBound,
}
