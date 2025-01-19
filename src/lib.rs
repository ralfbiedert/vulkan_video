//!
//! [![crates.io-badge]][crates.io-url]
//! [![docs.rs-badge]][docs.rs-url]
//! ![license-badge]
//!
//! # Vulkan Video
//!
//! Safe bindings to [Vulkan Video](https://www.khronos.org/blog/an-introduction-to-vulkan-video) via [ash](https://github.com/ash-rs/ash).
//!
//! - Rust-only GPU-accelerated Vulkan Video (we don't depend on FFMPEG, NVDEC, ...)
//! - Builds everywhere ash builds, minimal dependencies
//! - Exposes all<sup>†</sup> decode / encode operations supported by Vulkan (e.g., H.264, H.265, ...)
//! - Exposes compute functionality for post-processing
//! - Import / export foreign memory for interop<sup>†</sup>
//!
//! <sup>†</sup> Right now the code is a hot mess, this needs much more work to be useful.
//!
//! ## Status
//!
//! - **January 6th, 2025** - Re-activated for current `ash`; still won't work on your machine.
//! - **October 1st, 2023** - First 'proof of concept', as it can only decode one H.264 frame on the author's graphics card, and is many weeks away from being useful.
//!
//!
//! ## FAQ
//!
//! - **I'm getting weird errors**
//!
//!     We **STRONGLY** recommend you install the [Vulkan SDK](https://vulkan.lunarg.com/) and
//!     make sure the validation layer is available. In fact, some of the test cases
//!     require it and will error otherwise.
//!
//!     Apart from that, this needs much more work to initialize on various GPUs, help and PRs
//!     would be greatly appreciated.   
//!
//! - **Feature X is missing or broken, will you fix it?**
//!
//!     Right now I only have time to implement what I need. However, I will gladly accept PRs.
//!
//! - **How can I interop with DirectX, Torch, CUDA ...**
//!
//!     The idea is to support [Vulkan external memory](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VK_KHR_external_memory.html), and expose additional (feature-gated) APIs to ingest or export to external device memory.
//!     The exact details how to best interface with each external API are work-in-progress. Based on some initial tinkering zero-copy GPU-memory interop with other Vulkan instances is probably going to be 'easyish', with Torch probably 'hard'.
//!
//! - **Can I use this to decode MP4?**
//!
//!     We probably won't add container support to the core library. Instead you'd use another crate to parse your MP4 (or similar), and then feed the H.26x frames into this library.
//!
//! - **Why don't you run unit tests on CI?**
//!
//!     Support for Vulkan (Vulkan video in particular) on CIs is super flaky. Suggestions how to improve this are welcome!
//!
//! - **What's your UB policy?**
//!
//!     All Rust code in here should be safe and must never cause undefined behavior (UB). If you find anything that could cause UB, please file an issue.
//!     That said, right now most functions don't check their arguments properly and it might be easy to submit operations to Vulkan that cause fishy behavior.
//!     Also there is a chance that a bad compute shader could mess things up through the Vulkan backdoor. Whether that means all shader invocations should be `unsafe` I'm not yet sure, as that is similar to the `/proc/self/mem` file I/O issue in vanilla Rust.
//!
//!
//! ## Contributing
//!
//! PRs are very welcome. Feel free to submit trivial PRs right away. Architectural issues should be discussed first, but are also greatly appreciated.
//!
//!
//! ## License
//!
//! - BSD 2-Clause, Ralf Biedert
//!
//! [crates.io-badge]: https://img.shields.io/crates/v/vulkan_video.svg
//! [crates.io-url]: https://crates.io/crates/vulkan_video
//! [license-badge]: https://img.shields.io/badge/license-BSD2-blue.svg
//! [docs.rs-badge]: https://docs.rs/vulkan_video/badge.svg
//! [docs.rs-url]: https://docs.rs/vulkan_video/
//!
mod allocation;
pub(crate) mod commandbuffer;
mod device;
mod error;
mod instance;

pub mod ops;
mod physicaldevice;
mod queue;
pub mod resources;
pub mod shader;
pub mod video;
mod video_instance;

pub use allocation::Allocation;
pub use commandbuffer::CommandBuffer;
pub use device::Device;
pub use error::{Error, Variant};
pub use instance::{Instance, InstanceInfo};
pub use physicaldevice::{HeapInfos, PhysicalDevice, QueueFamilyInfos};
pub use queue::Queue;
