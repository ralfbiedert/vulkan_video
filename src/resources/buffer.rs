use crate::allocation::{Allocation, AllocationShared};
use crate::device::DeviceShared;
use crate::error::Error;
use crate::video::h264::H264StreamInspector;
use ash::vk;
use ash::vk::{
    BufferCreateInfo, BufferUsageFlags, DeviceSize, ExternalMemoryBufferCreateInfo, ExternalMemoryHandleTypeFlags, MappedMemoryRange,
    MemoryMapFlags, WHOLE_SIZE,
};
use std::ffi::c_void;
use std::sync::Arc;

/// Specifies how to crate a [`Buffer`](Buffer).
#[derive(Debug, Default, Clone)]
pub struct BufferInfo {
    size: u64,
    alignment: Option<u64>,
    offset: Option<u64>,
}

impl BufferInfo {
    pub fn new() -> Self {
        Self {
            size: 0,
            alignment: None,
            offset: None,
        }
    }

    pub fn size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment.into();
        self
    }

    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = offset.into();
        self
    }
}

pub(crate) struct BufferShared {
    shared_device: Arc<DeviceShared>,
    shared_allocation: Arc<AllocationShared>,
    device_buffer: vk::Buffer,
    buffer_info: BufferInfo,
}

impl BufferShared {
    pub fn new(shared_allocation: Arc<AllocationShared>, buffer_info: &BufferInfo) -> Result<Self, Error> {
        let shared_device = shared_allocation.device();
        let native_device = shared_device.native();

        let usage = BufferUsageFlags::STORAGE_BUFFER
            | BufferUsageFlags::TRANSFER_DST
            | BufferUsageFlags::TRANSFER_SRC
            | BufferUsageFlags::UNIFORM_BUFFER;

        unsafe {
            let buffer_create_info = BufferCreateInfo::default().size(buffer_info.size).usage(usage);

            let device_buffer = native_device.create_buffer(&buffer_create_info, None)?;
            let device_memory = shared_allocation.native();
            let offset = buffer_info.offset.unwrap_or(0);

            native_device.bind_buffer_memory(device_buffer, device_memory, offset)?;

            Ok(Self {
                shared_device,
                shared_allocation,
                device_buffer,
                buffer_info: buffer_info.clone(),
            })
        }
    }

    pub fn new_video_decode(
        shared_allocation: Arc<AllocationShared>,
        buffer_info: &BufferInfo,
        stream_inspector: &H264StreamInspector,
    ) -> Result<Self, Error> {
        let shared_device = shared_allocation.device();
        let native_device = shared_device.native();

        let usage = BufferUsageFlags::STORAGE_BUFFER
            | BufferUsageFlags::TRANSFER_DST
            | BufferUsageFlags::TRANSFER_SRC
            | BufferUsageFlags::VIDEO_DECODE_SRC_KHR
            | BufferUsageFlags::VIDEO_DECODE_DST_KHR;
        // | BufferUsageFlags::VIDEO_ENCODE_DST_KHR
        // | BufferUsageFlags::VIDEO_ENCODE_SRC_KHR;

        let mut profiles = stream_inspector.profiles();

        unsafe {
            let profile_infos = &mut profiles.as_mut().get_unchecked_mut().list;

            let buffer_create_info = BufferCreateInfo::default()
                .size(buffer_info.size)
                .usage(usage)
                .push_next(profile_infos);

            let device_buffer = native_device.create_buffer(&buffer_create_info, None)?;
            let device_memory = shared_allocation.native();
            let offset = buffer_info.offset.unwrap_or(0);

            native_device.bind_buffer_memory(device_buffer, device_memory, offset)?;

            Ok(Self {
                shared_device,
                shared_allocation,
                device_buffer,
                buffer_info: buffer_info.clone(),
            })
        }
    }

    pub fn external(shared_allocation: Arc<AllocationShared>, pointer: *mut c_void, buffer_info: &BufferInfo) -> Result<Self, Error> {
        let shared_device = shared_allocation.device();
        let native_device = shared_device.native();

        let usage = BufferUsageFlags::STORAGE_BUFFER
            | BufferUsageFlags::TRANSFER_DST
            | BufferUsageFlags::TRANSFER_SRC
            | BufferUsageFlags::UNIFORM_BUFFER;

        let mut eee = ExternalMemoryBufferCreateInfo::default().handle_types(ExternalMemoryHandleTypeFlags::OPAQUE_WIN32);

        unsafe {
            let buffer_create_info = BufferCreateInfo::default().size(buffer_info.size).usage(usage).push_next(&mut eee);

            let device_buffer = native_device.create_buffer(&buffer_create_info, None)?;
            let device_memory = shared_allocation.native();
            let offset = buffer_info.offset.unwrap_or(0);

            native_device.bind_buffer_memory(device_buffer, device_memory, offset)?;

            Ok(Self {
                shared_device,
                shared_allocation,
                device_buffer,
                buffer_info: buffer_info.clone(),
            })
        }
    }

    pub fn upload(&self, data: &[u8]) -> Result<(), Error> {
        let native_device = self.shared_device.native();
        let device_memory = self.shared_allocation.native();
        let offset = self.buffer_info.offset.unwrap_or(0);

        unsafe {
            let mapped_pointer = native_device.map_memory(device_memory, offset, WHOLE_SIZE, MemoryMapFlags::empty())?;

            std::ptr::copy_nonoverlapping::<u8>(data.as_ptr(), mapped_pointer.cast(), data.len());

            let mapped_range = MappedMemoryRange::default().size(WHOLE_SIZE).memory(device_memory).offset(offset);
            let mapped_range_slice = &[mapped_range];
            let rval = native_device.flush_mapped_memory_ranges(mapped_range_slice);

            native_device.unmap_memory(device_memory);

            rval?;
        }

        Ok(())
    }

    pub fn download_into(&self, target: &mut [u8]) -> Result<(), Error> {
        let native_device = self.shared_device.native();
        let device_memory = self.shared_allocation.native();
        let offset = self.buffer_info.offset.unwrap_or(0);

        unsafe {
            let len_bytes = target.len() as DeviceSize;
            let flags = MemoryMapFlags::empty();
            let mapped_pointer = native_device.map_memory(device_memory, offset, len_bytes, flags)?;

            // // DO I NEED THIS HERE?
            // let mapped_range = MappedMemoryRange::default().size(len_bytes).memory(device_memory);
            // let mapped_range_slice = &[mapped_range];
            // let rval = native_device.flush_mapped_memory_ranges(mapped_range_slice);

            std::ptr::copy_nonoverlapping::<u8>(mapped_pointer.cast(), target.as_mut_ptr(), len_bytes as usize);

            native_device.unmap_memory(device_memory);
        }

        Ok(())
    }

    pub fn size(&self) -> u64 {
        self.buffer_info.size
    }

    pub(crate) fn native(&self) -> vk::Buffer {
        self.device_buffer
    }

    pub(crate) fn device(&self) -> Arc<DeviceShared> {
        self.shared_device.clone()
    }
}

impl Drop for BufferShared {
    fn drop(&mut self) {
        let device = self.shared_device.native();

        unsafe {
            device.destroy_buffer(self.device_buffer, None);
        }
    }
}

/// A 1-dimensional memory block, usually on the GPU.
pub struct Buffer {
    shared: Arc<BufferShared>,
}

impl Buffer {
    pub fn new(allocation: &Allocation, info: &BufferInfo) -> Result<Self, Error> {
        let buffer_shared = BufferShared::new(allocation.shared(), info)?;

        Ok(Self {
            shared: Arc::new(buffer_shared),
        })
    }

    pub fn new_video_decode(allocation: &Allocation, info: &BufferInfo, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let buffer_shared = BufferShared::new_video_decode(allocation.shared(), info, stream_inspector)?;

        Ok(Self {
            shared: Arc::new(buffer_shared),
        })
    }

    pub fn external(allocation: &Allocation, pointer: *mut c_void, info: &BufferInfo) -> Result<Self, Error> {
        let buffer_shared = BufferShared::external(allocation.shared(), pointer, info)?;

        Ok(Self {
            shared: Arc::new(buffer_shared),
        })
    }

    pub fn size(&self) -> u64 {
        self.shared.size()
    }

    #[allow(unused)]
    pub(crate) fn shared(&self) -> Arc<BufferShared> {
        self.shared.clone()
    }

    pub fn upload(&self, data: &[u8]) -> Result<(), Error> {
        self.shared.upload(data)
    }

    pub fn download_into(&self, target: &mut [u8]) -> Result<(), Error> {
        self.shared.download_into(target)
    }
}

#[cfg(test)]
mod test {
    use crate::allocation::Allocation;
    use crate::device::Device;
    use crate::error;
    use crate::error::{Error, Variant};
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;
    use crate::resources::buffer::BufferInfo;
    use crate::resources::Buffer;
    use crate::video::h264::H264StreamInspector;

    #[test]
    #[cfg(not(miri))]
    fn crate_buffer() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let host_visible = physical_device
            .heap_infos()
            .any_host_visible()
            .ok_or_else(|| error!(Variant::HeapNotFound))?;
        let device = Device::new(&physical_device)?;
        let allocation = Allocation::new(&device, 16 * 1024, host_visible)?;
        let buffer_info = BufferInfo::new().size(1024).alignment(0).offset(0);

        _ = Buffer::new(&allocation, &buffer_info)?;

        Ok(())
    }

    #[test]
    #[cfg(not(miri))]
    fn crate_buffer_video() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let device_local = physical_device
            .heap_infos()
            .any_device_local()
            .ok_or_else(|| error!(Variant::HeapNotFound))?;
        let allocation = Allocation::new(&device, 16 * 1024, device_local)?;
        let buffer_info = BufferInfo::new().size(1024).alignment(0).offset(0);
        let h264inspector = H264StreamInspector::new();

        _ = Buffer::new_video_decode(&allocation, &buffer_info, &h264inspector)?;

        Ok(())
    }

    #[test]
    #[cfg(not(miri))]
    fn upload_download() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let host_visible = physical_device
            .heap_infos()
            .any_host_visible()
            .ok_or_else(|| error!(Variant::HeapNotFound))?;
        let allocation = Allocation::new(&device, 16 * 1024, host_visible)?;
        let buffer_info = BufferInfo::new().size(1024).alignment(0).offset(0);

        let buffer = Buffer::new(&allocation, &buffer_info)?;
        buffer.upload(&[1; 1024])?;

        let mut target = vec![0; 1024];
        buffer.download_into(&mut target)?;

        assert_eq!(target[0], 1);
        assert_eq!(target[1023], 1);

        Ok(())
    }
}
