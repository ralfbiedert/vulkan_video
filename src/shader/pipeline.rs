use crate::device::{Device, DeviceShared};
use crate::error;
use crate::error::{Error, Variant};
use crate::shader::parameters::ParametersShared;
use crate::shader::shader::{Shader, ShaderShared};
use crate::shader::ShaderParameterSet;
use ash::vk::{
    ComputePipelineCreateInfo, PipelineCache, PipelineLayout, PipelineLayoutCreateInfo, PipelineShaderStageCreateInfo, ShaderStageFlags,
};

#[expect(unused)]
pub(crate) struct PipelineShared<'a,T> {
    shared_device: &'a DeviceShared<'a>,
    shared_shader: &'a ShaderShared<'a,T>,
    shared_parameters: &'a ParametersShared<'a,T>,
    native_layout: PipelineLayout,
    native_pipeline: ash::vk::Pipeline,
}

impl<'a,T: ShaderParameterSet> PipelineShared<'a,T> {
    pub(crate) fn new(shared_device: &'a DeviceShared<'a>, shared_shader: &'a ShaderShared<T>) -> Result<Self, Error> {
        let native_device = shared_device.native();
        let shared_parameters = shared_shader.parameters();

        // TODO!!!
        // let push_constant = PushConstantRange::default()
        //     .offset(0)
        //     .size(4)
        //     .stage_flags(ShaderStageFlags::COMPUTE);
        //
        // let push_constants = [push_constant];
        let layouts = [shared_parameters.native_layout()];

        let pipeline_layout = PipelineLayoutCreateInfo::default().set_layouts(&layouts);

        let pipeline_shader_stage = PipelineShaderStageCreateInfo::default()
            .stage(ShaderStageFlags::COMPUTE)
            .module(shared_shader.native())
            .name(shared_shader.entry_point());

        unsafe {
            let native_layout = native_device.create_pipeline_layout(&pipeline_layout, None)?;

            let pipeline_info = ComputePipelineCreateInfo::default()
                .stage(pipeline_shader_stage)
                .layout(native_layout);

            let pipeline_infos = [pipeline_info];

            let native_pipeline = match native_device.create_compute_pipelines(PipelineCache::null(), &pipeline_infos, None) {
                Ok(mut pipelines) => pipelines.pop().ok_or_else(|| error!(Variant::NoComputePipeline))?,
                Err((_, e)) => {
                    native_device.destroy_pipeline_layout(native_layout, None);
                    return Err(error!(Variant::Vulkan(e)));
                }
            };

            Ok(Self {
                shared_device,
                shared_shader,
                shared_parameters,
                native_layout,
                native_pipeline,
            })
        }
    }

    pub(crate) fn parameters(&self) -> &ParametersShared<T> {
        &self.shared_parameters
    }
}

impl<'a,T> PipelineShared<'a,T> {
    pub(crate) fn native(&self) -> ash::vk::Pipeline {
        self.native_pipeline
    }

    pub(crate) fn layout(&self) -> ash::vk::PipelineLayout {
        self.native_layout
    }

    pub(crate) fn device(&self) -> &DeviceShared {
        &self.shared_device
    }
}

impl<'a,T> Drop for PipelineShared<'a,T> {
    fn drop(&mut self) {
        let native_device = self.shared_device.native();

        unsafe {
            native_device.destroy_pipeline(self.native_pipeline, None);
            native_device.destroy_pipeline_layout(self.native_layout, None);
        }
    }
}

/// Configuration how exactly a [Shader](Shader) should be invoked.
#[allow(unused)]
pub struct Pipeline<'a,T: ShaderParameterSet> {
    shared: PipelineShared<'a,T>,
}

impl<'a,T: ShaderParameterSet> Pipeline<'a,T> {
    pub fn new(device: &'a Device, shader: &'a Shader<T>) -> Result<Self, Error> {
        let shared = PipelineShared::new(device.shared(), shader.shared())?;

        Ok(Self { shared })
    }

    #[allow(unused)]
    pub(crate) fn shared(&self) -> &PipelineShared<T> {
        &self.shared
    }

    #[allow(unused)]
    pub(crate) fn layout(&self) -> ash::vk::PipelineLayout {
        self.shared.layout()
    }
}

#[cfg(test)]
mod test {
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;
    use crate::resources::Buffer;
    use crate::shader::{Parameters, Pipeline, Shader};

    #[test]
    #[cfg(not(miri))]
    fn create_pipeline() -> Result<(), Error> {
        let shader_code = include_bytes!("../../tests/shaders/compiled/hello_world.spv");

        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let parameters = Parameters::<(&Buffer, &Buffer, &Buffer)>::new(&device)?;
        let shader = Shader::new(&device, shader_code, "main", &parameters)?;

        _ = Pipeline::new(&device, &shader)?;

        Ok(())
    }
}
