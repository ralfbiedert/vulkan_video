use std::marker::PhantomData;
use std::sync::Arc;

use ash::vk::{DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType, ShaderStageFlags};

use crate::device::{Device, DeviceShared};
use crate::error::Error;
use crate::resources::{Buffer, ImageView};

pub enum ParameterType {
    Buffer {
        native: ash::vk::Buffer,
        size: u64,
    },
    ImageView {
        native_view: ash::vk::ImageView,
        native_image: ash::vk::Image,
    },
}

pub trait ShaderParameter {
    fn parameter_type(&self) -> ParameterType;

    fn descrtiptor_type() -> DescriptorType;
}
impl ShaderParameter for Buffer {
    fn parameter_type(&self) -> ParameterType {
        ParameterType::Buffer {
            native: self.shared().native(),
            size: self.size(),
        }
    }

    fn descrtiptor_type() -> DescriptorType {
        DescriptorType::STORAGE_BUFFER
    }
}

impl ShaderParameter for ImageView {
    fn parameter_type(&self) -> ParameterType {
        let native_image = self.native_image();
        let native_view = self.native();

        ParameterType::ImageView { native_view, native_image }
    }

    fn descrtiptor_type() -> DescriptorType {
        DescriptorType::STORAGE_IMAGE
    }
}

pub trait ShaderParameterSet {
    fn parameter_types(&self) -> Vec<ParameterType>;

    fn descriptor_types() -> Vec<DescriptorType>;
}

impl ShaderParameterSet for () {
    fn parameter_types(&self) -> Vec<ParameterType> {
        Vec::new()
    }

    fn descriptor_types() -> Vec<DescriptorType> {
        Vec::new()
    }
}

impl<T0> ShaderParameterSet for (&T0,)
where
    T0: ShaderParameter,
{
    fn parameter_types(&self) -> Vec<ParameterType> {
        vec![self.0.parameter_type()]
    }

    fn descriptor_types() -> Vec<DescriptorType> {
        vec![T0::descrtiptor_type()]
    }
}

impl<T0, T1, T2> ShaderParameterSet for (&T0, &T1, &T2)
where
    T0: ShaderParameter,
    T1: ShaderParameter,
    T2: ShaderParameter,
{
    fn parameter_types(&self) -> Vec<ParameterType> {
        vec![self.0.parameter_type(), self.1.parameter_type(), self.2.parameter_type()]
    }

    fn descriptor_types() -> Vec<DescriptorType> {
        vec![T0::descrtiptor_type(), T1::descrtiptor_type(), T2::descrtiptor_type()]
    }
}

pub(crate) struct ParametersShared<T> {
    shared_device: Arc<DeviceShared>,
    descriptor_set_layout: DescriptorSetLayout,
    _phantom: PhantomData<T>,
}

impl<T: ShaderParameterSet> ParametersShared<T> {
    pub fn new(shared_device: Arc<DeviceShared>) -> Result<Self, Error> {
        let native_device = shared_device.native();

        let descriptor_types = T::descriptor_types();
        let mut bindings = Vec::new();

        for (i, t) in descriptor_types.iter().enumerate() {
            let binding = DescriptorSetLayoutBinding::default()
                .binding(i as u32)
                .descriptor_count(1)
                .descriptor_type(*t)
                .stage_flags(ShaderStageFlags::COMPUTE);

            bindings.push(binding);
        }

        let create_info = DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        unsafe {
            let descriptor_set_layout = native_device.create_descriptor_set_layout(&create_info, None)?;

            Ok(Self {
                shared_device,
                descriptor_set_layout,
                _phantom: Default::default(),
            })
        }
    }

    pub fn native_layout(&self) -> DescriptorSetLayout {
        self.descriptor_set_layout
    }
}

impl<T> Drop for ParametersShared<T> {
    fn drop(&mut self) {
        unsafe {
            self.shared_device
                .native()
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

/// Holds parameter information for a [Shader](crate::shader::Shader).
pub struct Parameters<T: ShaderParameterSet> {
    shared: Arc<ParametersShared<T>>,
}

impl<T: ShaderParameterSet> Parameters<T> {
    pub fn new(device: &Device) -> Result<Self, Error> {
        let shared = ParametersShared::new(device.shared())?;

        Ok(Self { shared: Arc::new(shared) })
    }

    pub(crate) fn shared(&self) -> Arc<ParametersShared<T>> {
        self.shared.clone()
    }
}

#[cfg(test)]
mod test {
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;
    use crate::resources::Buffer;
    use crate::shader::parameters::Parameters;

    #[test]
    #[cfg(not(miri))]
    fn create_parameters() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;

        _ = Parameters::<(&Buffer,)>::new(&device)?;

        Ok(())
    }
}
