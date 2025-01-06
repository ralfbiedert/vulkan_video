use std::backtrace::Backtrace;
use std::ffi::NulError;
use std::fmt::{Display, Formatter};
use ash::LoadingError;
use ash::vk::CStrTooLargeForStaticArray;

#[derive(Debug)]
pub enum Variant {
    Nul(NulError),
    CStrTooLargeForStaticArray(CStrTooLargeForStaticArray),
    Loading(LoadingError),
    Vulkan(ash::vk::Result),
    NoVideoDevice,
    NoComputePipeline,
    NoCommandBuffer,
    HeapNotFound,
    QueueNotFound,
    ImageAlreadyBound,
}

pub struct Error {
    message: Option<String>,
    variant: Variant,
    backtrace: Backtrace,
}

impl Error {
    pub fn new(message: Option<String>, variant: Variant) -> Self {
        Self {
            message,
            variant,
            backtrace: Backtrace::capture(),
        }
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.message {
            Some(msg) => writeln!(f, "{}: {:?}", msg, self.variant)?,
            None => writeln!(f, "{:?}", self.variant)?,
        }

        writeln!(f, "Backtrace:\n{}", self.backtrace)
    }
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Print the error message (if any) and the variant
        match &self.message {
            Some(msg) => writeln!(f, "{}: {:?}", msg, self.variant),
            None => writeln!(f, "{:?}", self.variant),
        }?;

        // Use the stable `Display` implementation of `Backtrace`
        writeln!(f, "Backtrace:\n{}", self.backtrace)
    }
}

impl From<ash::vk::Result> for Error {
    fn from(e: ash::vk::Result) -> Self {
        Self {
            message: None,
            variant: Variant::Vulkan(e),
            backtrace: Backtrace::capture(),
        }
    }
}

impl From<NulError> for Error {
    fn from(e: NulError) -> Self {
        Self {
            message: None,
            variant: Variant::Nul(e),
            backtrace: Backtrace::capture(),
        }
    }
}

impl From<LoadingError> for Error {
    fn from(e: LoadingError) -> Self {
        Self {
            message: None,
            variant: Variant::Loading(e),
            backtrace: Backtrace::capture(),
        }
    }
}

impl From<CStrTooLargeForStaticArray> for Error {
    fn from(e: CStrTooLargeForStaticArray) -> Self {
        Self {
            message: None,
            variant: Variant::CStrTooLargeForStaticArray(e),
            backtrace: Backtrace::capture(),
        }
    }
}


#[macro_export]
macro_rules! error {
    ($variant:expr, $($args:tt)*) => {
        {
            let message = format!($($args)*);
            $crate::Error::new(Some(message), $variant)
        }
    };
    ($variant:expr) => {
        {
            $crate::Error::new(None, $variant)
        }
    };
}