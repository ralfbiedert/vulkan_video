use crate::device::Device;
use crate::error::Error;
use crate::shader::parameters::Parameters;
use crate::shader::ShaderParameterSet;
use ash::vk::{ShaderModule, ShaderModuleCreateInfo};
use std::ffi::{CStr, CString};

/// Some GPU program, mostly for postprocessing video frames.
pub struct Shader<'a, T> {
    shared_device: &'a Device<'a>,
    shared_parameters: &'a Parameters<'a, T>,
    shader_module: ShaderModule,
    entry_point: CString,
}

impl<'a, T: ShaderParameterSet> Shader<'a, T> {
    pub fn new(
        shared_device: &'a Device<'a>,
        spirv_code: &[u8],
        entry_point: &str,
        shared_parameters: &'a Parameters<'a, T>,
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

    pub(crate) fn parameters(&self) -> &Parameters<'_, T> {
        &self.shared_parameters
    }
}

impl<'a, T> Drop for Shader<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.shared_device.native().destroy_shader_module(self.shader_module, None);
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
