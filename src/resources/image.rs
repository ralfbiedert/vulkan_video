use crate::allocation::{Allocation, MemoryTypeIndex};
use ash::vk::{Extent3D, Format, ImageCreateInfo, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, SampleCountFlags};

use crate::device::Device;
use crate::error::Error;
use crate::video::h264::H264StreamInspector;

pub struct MemoryRequirements {
    size: u64,
    alignment: u64,
    memory_type_bits: u32,
}

impl MemoryRequirements {
    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn alignment(&self) -> u64 {
        self.alignment
    }

    pub fn any_heap(&self) -> MemoryTypeIndex {
        MemoryTypeIndex::new(self.memory_type_bits.trailing_zeros())
    }
}

/// Specifies how to crate an [`Image`](Image).
#[derive(Debug, Default, Clone)]
pub struct ImageInfo {
    format: Format,
    samples: SampleCountFlags,
    usage: ImageUsageFlags,
    mip_levels: u32,
    array_layers: u32,
    bind_offset: u64,
    image_type: ImageType,
    tiling: ImageTiling,
    extent: Extent3D,
    layout: ImageLayout,
}

impl ImageInfo {
    pub fn new() -> ImageInfo {
        Self::default()
    }

    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    pub fn samples(mut self, samples: SampleCountFlags) -> Self {
        self.samples = samples;
        self
    }

    pub fn usage(mut self, usage: ImageUsageFlags) -> Self {
        self.usage = usage;
        self
    }

    pub fn mip_levels(mut self, mip_levels: u32) -> Self {
        self.mip_levels = mip_levels;
        self
    }

    pub fn array_layers(mut self, array_layers: u32) -> Self {
        self.array_layers = array_layers;
        self
    }

    pub fn image_type(mut self, image_type: ImageType) -> Self {
        self.image_type = image_type;
        self
    }

    pub fn tiling(mut self, tiling: ImageTiling) -> Self {
        self.tiling = tiling;
        self
    }

    pub fn extent(mut self, extent: Extent3D) -> Self {
        self.extent = extent;
        self
    }

    pub fn get_extent(&self) -> Extent3D {
        self.extent
    }

    pub fn layout(mut self, layout: ImageLayout) -> Self {
        self.layout = layout;
        self
    }
}

struct ImageInner<'a> {
    device: &'a Device<'a>,
    native_image: ash::vk::Image,
    info: ImageInfo,
}

impl<'a> ImageInner<'a> {
    fn new(device: &'a Device<'a>, info: &ImageInfo) -> Result<Self, Error> {
        let native_device = device.native();

        let create_image = ImageCreateInfo::default()
            .format(info.format) // we got this from the videosession struct which listed this as teh format.
            .samples(info.samples)
            .usage(info.usage)
            .mip_levels(info.mip_levels)
            .array_layers(info.array_layers)
            .image_type(info.image_type)
            .tiling(info.tiling)
            .initial_layout(info.layout)
            // .push_next(&mut video_profile_list_info_khr)
            .extent(info.extent);

        unsafe {
            let native_image = native_device.create_image(&create_image, None)?;

            Ok(Self {
                device,
                native_image,
                info: info.clone(),
            })
        }
    }

    fn new_video_target(device: &'a Device<'a>, info: &ImageInfo, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let native_device = device.native();

        unsafe {
            let mut profiles = stream_inspector.profiles();
            let profiles_inner = profiles.as_mut().get_unchecked_mut();

            let create_image = ImageCreateInfo::default()
                .format(info.format) // we got this from the videosession struct which listed this as teh format.
                .samples(info.samples)
                .usage(info.usage)
                .mip_levels(info.mip_levels)
                .array_layers(info.array_layers)
                .image_type(info.image_type)
                .tiling(info.tiling)
                .initial_layout(info.layout)
                .push_next(&mut profiles_inner.list)
                .extent(info.extent);

            let native_image = native_device.create_image(&create_image, None)?;

            Ok(Self {
                device,
                native_image,
                info: info.clone(),
            })
        }
    }

    fn memory_requirement(&self) -> MemoryRequirements {
        let native_device = self.device.native();

        unsafe {
            let requirements = native_device.get_image_memory_requirements(self.native_image);

            MemoryRequirements {
                size: requirements.size,
                alignment: requirements.alignment,
                memory_type_bits: requirements.memory_type_bits,
            }
        }
    }
}

/// An `Image` that has yet to be bound.  Call .bind() to construct an `Image`.
pub struct UnboundImage<'a> {
    inner: ImageInner<'a>,
}

impl<'a> UnboundImage<'a> {
    pub fn new(device: &'a Device<'a>, info: &ImageInfo) -> Result<Self, Error> {
        let inner = ImageInner::new(device, info)?;
        Ok(Self { inner })
    }
    pub fn new_video_target(device: &'a Device<'a>, info: &ImageInfo, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let inner = ImageInner::new_video_target(device, info, stream_inspector)?;
        Ok(Self { inner })
    }

    #[must_use]
    pub fn bind(self, allocation: &'a Allocation<'a>) -> Result<Image<'a>, Error> {
        let inner = self.inner;
        let native_device = inner.device.native();
        let native_image = inner.native_image;
        let native_allocation = allocation.native();

        unsafe {
            native_device.bind_image_memory(native_image, native_allocation, inner.info.bind_offset)?;
        }

        Ok(Image { inner })
    }

    pub fn memory_requirement(&self) -> MemoryRequirements {
        self.inner.memory_requirement()
    }
}

/// A often 2D image, usually stored on the GPU.
pub struct Image<'a> {
    inner: ImageInner<'a>,
}

impl<'a> Image<'a> {
    pub(crate) fn native(&self) -> ash::vk::Image {
        self.inner.native_image
    }

    pub(crate) fn device(&self) -> &Device<'_> {
        &self.inner.device
    }

    pub(crate) fn info(&self) -> ImageInfo {
        self.inner.info.clone()
    }
}

impl<'a> Drop for ImageInner<'a> {
    fn drop(&mut self) {
        let native_device = self.device.native();

        unsafe {
            native_device.destroy_image(self.native_image, None);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::allocation::Allocation;
    use ash::vk::{Extent3D, Format, ImageTiling, ImageType, ImageUsageFlags, SampleCountFlags};

    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;
    use crate::resources::{ImageInfo, UnboundImage};

    #[test]
    #[cfg(not(miri))]
    fn crate_image() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let info = ImageInfo::new()
            .format(Format::G8_B8R8_2PLANE_420_UNORM)
            .samples(SampleCountFlags::TYPE_1)
            .usage(ImageUsageFlags::TRANSFER_SRC | ImageUsageFlags::TRANSFER_DST)
            .mip_levels(1)
            .array_layers(1)
            .image_type(ImageType::TYPE_2D)
            .tiling(ImageTiling::OPTIMAL)
            .extent(Extent3D::default().width(512).height(512).depth(1));
        let image = UnboundImage::new(&device, &info)?;
        let heap_index = image.memory_requirement().any_heap();
        let allocation = Allocation::new(&device, 1024 * 1024, heap_index)?;

        _ = image.bind(&allocation)?;

        Ok(())
    }
}
