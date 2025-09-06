use crate::device::{Device, DeviceShared};
use crate::error::Error;
use crate::shader::parameters::{Parameters, ParametersShared};
use crate::shader::ShaderParameterSet;
use ash::vk::{ShaderModule, ShaderModuleCreateInfo};
use std::ffi::{CStr, CString};

#[allow(unused)]
pub(crate) struct ShaderShared<'a,T> {
    shared_device: &'a DeviceShared<'a>,
    shared_parameters: &'a ParametersShared<'a,T>,
    shader_module: ShaderModule,
    entry_point: CString,
}

impl<'a,T: ShaderParameterSet> ShaderShared<'a,T> {
    pub fn new(
        shared_device: &'a DeviceShared<'a>,
        spirv_code: &[u8],
        entry_point: &str,
        shared_parameters: &'a ParametersShared<'a,T>,
    ) -> Result<Self, Error> {
        let entry_point = CString::new(entry_point)?;

        let mut create_info = ShaderModuleCreateInfo::default();
        create_info.p_code = spirv_code.as_ptr().cast();
        create_info.code_size = spirv_code.len();

        unsafe {
            let shader_module = shared_device.native().create_shader_module(&create_info, None)?;

            Ok(Self {
                shared_device,
                shared_parameters,
                shader_module,
                entry_point,
            })
        }
    }

    pub(crate) fn native(&self) -> ShaderModule {
        self.shader_module
    }

    pub(crate) fn entry_point(&self) -> &CStr {
        &self.entry_point
    }

    pub(crate) fn parameters(&self) -> &ParametersShared<T> {
        &self.shared_parameters
    }
}

impl<'a,T> Drop for ShaderShared<'a,T> {
    fn drop(&mut self) {
        unsafe {
            self.shared_device.native().destroy_shader_module(self.shader_module, None);
        }
    }
}

/// Some GPU program, mostly for postprocessing video frames.
pub struct Shader<'a,T: ShaderParameterSet> {
    shared: ShaderShared<'a,T>,
}

impl<'a,T: ShaderParameterSet> Shader<'a,T> {
    pub fn new(device: &'a Device, spirv_code: &[u8], entry_point: &str, parameters: &'a Parameters<T>) -> Result<Self, Error> {
        let shared = ShaderShared::<T>::new(device.shared(), spirv_code, entry_point, parameters.shared())?;

        Ok(Self { shared })
    }

    pub(crate) fn shared(&self) -> &ShaderShared<T> {
        &self.shared
    }

    #[allow(unused)]
    pub fn entry_point(&self) -> &CStr {
        self.shared.entry_point()
    }

    #[allow(unused)]
    pub(crate) fn parameters(&self) -> &ParametersShared<T> {
        self.shared().parameters()
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
    use crate::shader::shader::Shader;

    #[test]
    #[cfg(not(miri))]
    fn load_shader() -> Result<(), Error> {
        let shader_code = include_bytes!("../../tests/shaders/compiled/hello_world.spv");

        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let parameters = Parameters::<(&Buffer,)>::new(&device)?;

        _ = Shader::new(&device, shader_code, "main", &parameters)?;

        Ok(())
    }
}
