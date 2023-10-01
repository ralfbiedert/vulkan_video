use std::cell::RefCell;
use std::sync::Arc;

use crate::allocation::{Allocation, AllocationShared};
use ash::vk::{Extent3D, Format, ImageCreateInfo, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, SampleCountFlags};

use crate::device::{Device, DeviceShared};
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

    pub fn any_heap(&self) -> u32 {
        self.memory_type_bits.trailing_zeros()
    }
}

/// Specifies how to crate an [`Image`](Image).
#[derive(Debug, Clone)]
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
        Self {
            format: Default::default(),
            samples: Default::default(),
            usage: Default::default(),
            mip_levels: 0,
            array_layers: 0,
            bind_offset: 0,
            image_type: Default::default(),
            tiling: Default::default(),
            extent: Default::default(),
            layout: Default::default(),
        }
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

pub(crate) struct ImageShared {
    shared_device: Arc<DeviceShared>,
    shared_allocation: RefCell<Option<Arc<AllocationShared>>>,
    native_image: ash::vk::Image,
    info: ImageInfo,
}

impl ImageShared {
    fn new(shared_device: Arc<DeviceShared>, info: &ImageInfo) -> Result<Self, Error> {
        let native_device = shared_device.native();

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
                shared_device,
                shared_allocation: RefCell::new(None),
                native_image,
                info: info.clone(),
            })
        }
    }

    fn new_video_target(shared_device: Arc<DeviceShared>, info: &ImageInfo, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let native_device = shared_device.native();

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
                shared_device,
                shared_allocation: RefCell::new(None),
                native_image,
                info: info.clone(),
            })
        }
    }

    pub fn bind(&self, shared_allocation: Arc<AllocationShared>) -> Result<(), Error> {
        let native_device = self.shared_device.native();
        let native_image = self.native_image;
        let native_allocation = shared_allocation.native();

        if self.shared_allocation.borrow().is_some() {
            return Err(Error::ImageAlreadyBound);
        }

        unsafe {
            native_device.bind_image_memory(native_image, native_allocation, self.info.bind_offset)?;

            self.shared_allocation.replace(Some(shared_allocation));

            Ok(())
        }
    }

    pub(crate) fn memory_requirement(&self) -> MemoryRequirements {
        let native_device = self.shared_device.native();

        unsafe {
            let requirements = native_device.get_image_memory_requirements(self.native_image);

            MemoryRequirements {
                size: requirements.size,
                alignment: requirements.alignment,
                memory_type_bits: requirements.memory_type_bits,
            }
        }
    }

    pub(crate) fn native(&self) -> ash::vk::Image {
        self.native_image.clone()
    }

    pub(crate) fn device(&self) -> Arc<DeviceShared> {
        self.shared_device.clone()
    }

    pub(crate) fn info(&self) -> ImageInfo {
        self.info.clone()
    }
}

impl Drop for ImageShared {
    fn drop(&mut self) {
        let native_device = self.shared_device.native();

        unsafe {
            native_device.destroy_image(self.native_image, None);
        }
    }
}

/// A often 2D image, usually stored on the GPU.
pub struct Image {
    shared: Arc<ImageShared>,
}

impl Image {
    pub fn new(device: &Device, info: &ImageInfo) -> Result<Self, Error> {
        let shared_device = ImageShared::new(device.shared(), info)?;

        Ok(Self {
            shared: Arc::new(shared_device),
        })
    }

    pub fn new_video_target(device: &Device, info: &ImageInfo, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let shared_device = ImageShared::new_video_target(device.shared(), info, stream_inspector)?;

        Ok(Self {
            shared: Arc::new(shared_device),
        })
    }

    pub fn bind(self, allocation: &Allocation) -> Result<Self, Error> {
        self.shared.bind(allocation.shared())?;
        Ok(self)
    }

    pub fn memory_requirement(&self) -> MemoryRequirements {
        self.shared.memory_requirement()
    }

    pub(crate) fn shared(&self) -> Arc<ImageShared> {
        self.shared.clone()
    }
    pub(crate) fn native(&self) -> ash::vk::Image {
        self.shared.native()
    }
    pub(crate) fn device(&self) -> Arc<DeviceShared> {
        self.shared.shared_device.clone()
    }

    pub fn info(&self) -> ImageInfo {
        self.shared.info()
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
    use crate::resources::{Image, ImageInfo};

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
        let image = Image::new(&device, &info)?;
        let heap_index = image.memory_requirement().any_heap();
        let allocation = Allocation::new(&device, 1024 * 1024, heap_index)?;

        _ = image.bind(&allocation)?;

        Ok(())
    }
}
