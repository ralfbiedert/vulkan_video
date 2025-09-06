use crate::error::Error;
use ash::vk;
use ash::vk::{ApplicationInfo, InstanceCreateFlags, InstanceCreateInfo};
use std::ffi::CString;

/// Stores information (e.g., app name, version) about the current instance.
#[derive(Debug)]
pub struct InstanceInfo {
    app_name: CString,
    engine_name: CString,
    engine_version: u32,
    app_version: u32,
    validation: bool,
}

impl InstanceInfo {
    pub fn new() -> Self {
        Self {
            app_name: Default::default(),
            engine_name: Default::default(),
            engine_version: 0,
            app_version: 0,
            validation: false,
        }
    }

    pub fn app_name(mut self, app_name: &str) -> Result<Self, Error> {
        self.app_name = CString::new(app_name)?;
        Ok(self)
    }

    pub fn engine_name(mut self, engine_name: &str) -> Result<Self, Error> {
        self.engine_name = CString::new(engine_name)?;
        Ok(self)
    }

    pub fn engine_version(mut self, engine_version: u32) -> Self {
        self.engine_version = engine_version;
        self
    }

    pub fn app_version(mut self, app_version: u32) -> Self {
        self.app_version = app_version;
        self
    }

    /// Enables the Vulkan validation layer.
    ///
    /// # Errors
    ///
    /// Enabling this can cause initialization failures if the validation layers are not present.
    /// You probably need the Vulkan SDK installed.
    pub fn validation(mut self, validation: bool) -> Self {
        self.validation = validation;
        self
    }
}

impl Default for InstanceInfo {
    fn default() -> Self {
        InstanceInfo::new()
    }
}

pub(crate) struct InstanceShared {
    instance: ash::Instance,
    entry: ash::Entry,
}

impl InstanceShared {
    pub fn new(info: &InstanceInfo) -> Result<Self, Error> {
        let vulkan_version = vk::make_api_version(0, 1, 3, 0);
        let debug_layers = [c"VK_LAYER_KHRONOS_validation".as_ptr().cast()];
        let enabled_layers = if info.validation { debug_layers.as_slice() } else { &[] };
        let instance_extensions = [c"VK_KHR_portability_enumeration".as_ptr().cast()];

        let app_info = ApplicationInfo::default()
            .application_name(&info.app_name)
            .application_version(info.app_version)
            .engine_name(&info.engine_name)
            .engine_version(info.engine_version)
            .api_version(vulkan_version);

        let instance_create_info = InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(enabled_layers)
            .enabled_extension_names(&instance_extensions)
            .flags(InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);

        unsafe {
            let entry = ash::Entry::load()?;
            let instance = entry.create_instance(&instance_create_info, None)?;
            Ok(Self { instance, entry })
        }
    }

    pub fn native(&self) -> &ash::Instance {
        &self.instance
    }

    pub fn native_entry(&self) -> &ash::Entry {
        &self.entry
    }
}

impl Drop for InstanceShared {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

/// The Vulkan driver instance, **start here**.
pub struct Instance {
    shared: InstanceShared,
}

impl Instance {
    pub fn new(info: &InstanceInfo) -> Result<Self, Error> {
        Ok(Self {
            shared: InstanceShared::new(info)?,
        })
    }

    pub(crate) fn shared(&self) -> &InstanceShared {
        &self.shared
    }
}

#[cfg(test)]
mod test {
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo, InstanceShared};

    #[test]
    #[cfg(not(miri))]
    fn create_shared_instance() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);

        _ = InstanceShared::new(&instance_info)?;

        Ok(())
    }

    #[test]
    #[cfg(not(miri))]
    fn create_instance() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);

        _ = Instance::new(&instance_info)?;

        Ok(())
    }
}
