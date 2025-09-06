use ash::vk::{Format, ImageAspectFlags, ImageSubresourceRange, ImageViewCreateInfo, ImageViewType};

use crate::device::DeviceShared;
use crate::error::Error;
use crate::resources::image::ImageShared;
use crate::resources::Image;

/// Specifies how to crate an  [`ImageView`](ImageView).
#[derive(Clone, Debug, Default)]
pub struct ImageViewInfo {
    format: Format,
    image_view_type: ImageViewType,
    aspect_mask: ImageAspectFlags,
    layer_count: u32,
    level_count: u32,
}

impl ImageViewInfo {
    pub fn new() -> ImageViewInfo {
        Self::default()
    }

    pub fn format(mut self, format: Format) -> Self {
        self.format = format;
        self
    }

    pub fn image_view_type(mut self, image_view_type: ImageViewType) -> Self {
        self.image_view_type = image_view_type;
        self
    }

    pub fn aspect_mask(mut self, aspect_mask: ImageAspectFlags) -> Self {
        self.aspect_mask = aspect_mask;
        self
    }

    pub fn layer_count(mut self, layer_count: u32) -> Self {
        self.layer_count = layer_count;
        self
    }

    pub fn level_count(mut self, level_count: u32) -> Self {
        self.level_count = level_count;
        self
    }
}

pub(crate) struct ImageViewShared<'a> {
	shared_image: &'a ImageShared<'a>,
    shared_device: &'a DeviceShared<'a>,
    native_view: ash::vk::ImageView,
}

impl<'a> ImageViewShared<'a> {
	pub fn new(shared_image: &'a ImageShared<'a>, info: &ImageViewInfo) -> Result<Self, Error> {
        let shared_device = shared_image.device();

        let native_image = shared_image.native();
        let native_device = shared_device.native();

        let srr = ImageSubresourceRange::default()
            .aspect_mask(info.aspect_mask)
            .layer_count(info.layer_count)
            .level_count(info.level_count);

        let create_image_view = ImageViewCreateInfo::default()
            .image(native_image)
            .subresource_range(srr)
            .format(info.format)
            .view_type(info.image_view_type);

        unsafe {
            let native_view = native_device.create_image_view(&create_image_view, None)?;

            Ok(ImageViewShared {
                shared_device,
                shared_image,
                native_view,
            })
        }
    }

    pub(crate) fn native(&self) -> ash::vk::ImageView {
        self.native_view
    }

    pub(crate) fn image(&self) -> &ImageShared {
        &self.shared_image
    }
}

impl<'a> Drop for ImageViewShared<'a> {
    fn drop(&mut self) {
        let native_device = self.shared_device.native();

        unsafe {
            native_device.destroy_image_view(self.native_view, None);
        }
    }
}

/// View of an [`Image`](Image).
pub struct ImageView<'a> {
    shared_view: ImageViewShared<'a>,
}

impl<'a> ImageView<'a> {
    pub fn new(image: &'a Image<'a>, info: &ImageViewInfo) -> Result<Self, Error> {
        let shared_view = ImageViewShared::new(image.shared(), info)?;

        Ok(Self {
            shared_view: shared_view,
        })
    }

    pub(crate) fn shared(&self) -> &ImageViewShared {
        &self.shared_view
    }

    pub(crate) fn native(&self) -> ash::vk::ImageView {
        self.shared_view.native()
    }

    pub(crate) fn native_image(&self) -> ash::vk::Image {
        self.shared_view.shared_image.native()
    }
}

#[cfg(test)]
mod test {
    use crate::allocation::Allocation;
    use ash::vk::{Extent3D, Format, ImageAspectFlags, ImageTiling, ImageType, ImageUsageFlags, ImageViewType, SampleCountFlags};

    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;
    use crate::resources::{Image, ImageInfo, ImageView, ImageViewInfo};

    #[test]
    #[cfg(not(miri))]
    fn crate_image_view() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let image_info = ImageInfo::new()
            .format(Format::R8_UNORM)
            .samples(SampleCountFlags::TYPE_1)
            .usage(ImageUsageFlags::TRANSFER_SRC | ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED)
            .mip_levels(1)
            .array_layers(1)
            .image_type(ImageType::TYPE_2D)
            .tiling(ImageTiling::OPTIMAL)
            .extent(Extent3D::default().width(512).height(512).depth(1));

        let image = Image::new(&device, &image_info)?;
        let heap_type = image.memory_requirement().any_heap();
        let allocation = Allocation::new(&device, 1024 * 1024, heap_type)?;

        let image = image.bind(&allocation)?;

        let image_view_info = ImageViewInfo::new()
            .aspect_mask(ImageAspectFlags::COLOR)
            .format(Format::R8_UNORM)
            .image_view_type(ImageViewType::TYPE_2D)
            .layer_count(1)
            .level_count(1);

        _ = ImageView::new(&image, &image_view_info)?;

        Ok(())
    }
}
