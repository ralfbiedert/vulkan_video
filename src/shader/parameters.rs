use std::marker::PhantomData;

use ash::vk::{DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType, ShaderStageFlags};

use crate::device::Device;
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
impl<'a> ShaderParameter for Buffer<'a> {
    fn parameter_type(&self) -> ParameterType {
        ParameterType::Buffer {
            native: self.native(),
            size: self.size(),
        }
    }

    fn descrtiptor_type() -> DescriptorType {
        DescriptorType::STORAGE_BUFFER
    }
}

impl<'a> ShaderParameter for ImageView<'a> {
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

/// Holds parameter information for a [Shader](crate::shader::Shader).
pub struct Parameters<'a, T> {
    shared_device: &'a Device<'a>,
    descriptor_set_layout: DescriptorSetLayout,
    _phantom: PhantomData<T>,
}

impl<'a, T: ShaderParameterSet> Parameters<'a, T> {
    pub fn new(shared_device: &'a Device<'a>) -> Result<Self, Error> {
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

impl<'a, T> Drop for Parameters<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.shared_device
                .native()
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;
    use crate::resources::Buffer;
    use crate::shader::Parameters;

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
