use std::ffi::CStr;

use ash::{
    Instance,
    vk::{self},
};

use crate::renderer::presentation::VKSurface;

/// Function for Checking Requirements
type ReqFn<'a> = Box<dyn Fn(vk::PhysicalDevice, &Instance, &VKSurface) -> bool + 'a>;

/// Struct for holding and testing Device Requirements
/// Example Use:
/// ```ignore
/// let physical_device = ...;
/// let DeviceRequirements = DeviceRequirements::default().push_ext(ash::khr::dynamic_rendering::NAME);
/// println!("Compatible {:?}", DeviceRequirements.check_device(physical_device));
/// ```
pub struct VKDeviceRequirements<'a> {
    pub required_extensions: Vec<&'static CStr>,
    pub device_extended_info: Vec<Box<dyn vk::ExtendsDeviceCreateInfo + 'a>>,
    pub requirement_functions: Vec<ReqFn<'a>>,
    pub queue_requests: VKQueueRequests,
    pub min_api_version: u32,
    pub api_version: u32,
}

impl<'a> VKDeviceRequirements<'a> {
    /// Adds a Vulkan extensions name to the requirements
    pub fn push_ext(mut self, ext_name: &'static CStr) -> Self {
        self.required_extensions.push(ext_name);
        self
    }

    /// Adds Structures that extend the creation of logical Devices to the requirements
    /// This is so they can be used on logical Device creation
    pub fn push_info<T>(mut self, dev_ext_info: T) -> Self
    where
        T: vk::ExtendsDeviceCreateInfo + 'a,
    {
        self.device_extended_info.push(Box::new(dev_ext_info));
        self
    }

    /// Adds a `fn(vk::PhysicalDevice, &Instance, &VKSurface) -> bool` to the device compatibility check process
    /// fn must return whether device meats functions requirements.
    pub fn push_fn<F>(mut self, fn_test: F) -> Self
    where
        F: Fn(vk::PhysicalDevice, &Instance, &VKSurface) -> bool + 'a,
    {
        self.requirement_functions.push(Box::new(fn_test));
        self
    }

    /// add queue request
    pub fn add_queue_request(mut self, queue_request: VKQueueRequest) -> Self {
        debug_assert!(
            !self
                .queue_requests
                .iter()
                .any(|r| r.role_id == queue_request.role_id),
            "duplicate queue role_id: {}",
            queue_request.role_id
        );
        self.queue_requests.push(queue_request);
        self
    }

    /// add many queue requests
    pub fn add_queue_requests<I>(mut self, queue_requests: I) -> Self
    where
        I: IntoIterator<Item = VKQueueRequest>,
    {
        for request in queue_requests {
            debug_assert!(
                !self
                    .queue_requests
                    .iter()
                    .any(|r| r.role_id == request.role_id),
                "duplicate queue role_id detected: {}",
                request.role_id
            );

            self.queue_requests.push(request);
        }
        self
    }

    // set the Vulkan instance API version
    pub fn instance_verion(mut self, version: u32) -> Self {
        self.api_version = version;
        self
    }

    // set the minimum required version for the device compatibility check process
    pub fn require_api_version(mut self, version: u32) -> Self {
        self.min_api_version = version;
        self
    }

    /// Checks if Physical Device is Compatible.
    // Maybe upgrade to -> Result Type as we currently treat less related errors as an incompatible device
    // Most of the errors are 'VKResult' errors retaining to memory issues unlikely at early initialisation.
    // TODO: Return Reason for Compatibility issue in Result With Custom Error Type
    pub fn device_compat(
        &self,
        physical_device: vk::PhysicalDevice,
        instance: &Instance,
        surface_requirment: &VKSurface,
    ) -> VKDeviceCompat {
        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };

        if device_properties.api_version.min(self.api_version) < self.min_api_version {
            return VKDeviceCompat::Incompatible;
        }

        let device_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(physical_device)
                .unwrap_or_default()
        };

        let device_extensions: Vec<&CStr> = device_extensions
            .iter()
            .map(|ext_prop| ext_prop.extension_name_as_c_str().unwrap_or_default())
            .collect();

        let has_extensions = self
            .required_extensions
            .iter()
            .all(|extensions| device_extensions.contains(extensions));

        let funcs_passes = self
            .requirement_functions
            .iter()
            .all(|func| func(physical_device, instance, surface_requirment));

        if !has_extensions || !funcs_passes {
            return VKDeviceCompat::Incompatible;
        }

        if self.queue_requests.is_empty() {
            return VKDeviceCompat::Compatable;
        } else {
            if let Some(queue_requests) =
                self.queue_requests
                    .fulfill_requests(physical_device, instance, surface_requirment)
            {
                return VKDeviceCompat::CompatibleWithQueues(queue_requests);
            } else {
                return VKDeviceCompat::Incompatible;
            }
        }
    }

    pub fn get_requirements(&self) -> &[&'static CStr] {
        self.required_extensions.as_slice()
    }

    pub fn get_requirements_raw(&self) -> Vec<*const std::ffi::c_char> {
        self.required_extensions
            .iter()
            .map(|req| req.as_ptr())
            .collect()
    }
}

impl Default for VKDeviceRequirements<'_> {
    fn default() -> Self {
        Self {
            required_extensions: Vec::new(),
            device_extended_info: Vec::new(),
            requirement_functions: Vec::new(),
            queue_requests: VKQueueRequests::default(),
            min_api_version: vk::API_VERSION_1_0,
            api_version: vk::API_VERSION_1_0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct VKQueueRequest {
    pub role_id: u32,
    pub required_queue_flags: vk::QueueFlags,
    pub excluded_queue_flags: vk::QueueFlags,
    pub require_surface_support: bool,
}

impl VKQueueRequest {
    /// Sets name of request for debug
    pub fn queue_role(mut self, role: u32) -> Self {
        self.role_id = role;
        self
    }
    /// Adds queue flag to the requirement
    pub fn require_queue_flag(mut self, queue_flag: vk::QueueFlags) -> Self {
        self.required_queue_flags |= queue_flag;
        self
    }

    /// Adds queue flag to be excluded
    pub fn exclude_queue_flag(mut self, queue_flag: vk::QueueFlags) -> Self {
        self.excluded_queue_flags |= queue_flag;
        self
    }

    /// Queue Requires Vulkan Surface support
    pub fn surface_support(mut self, support_req: bool) -> Self {
        self.require_surface_support = support_req;
        self
    }

    pub fn queue_compatible(
        &self,
        queue_index: u32,
        queue_prop: &vk::QueueFamilyProperties,
        physical_device: vk::PhysicalDevice,
        surface_requirment: &VKSurface,
        ignore_excl: bool,
    ) -> bool {
        (queue_prop.queue_flags.contains(self.required_queue_flags))
            && (ignore_excl || !queue_prop.queue_flags.intersects(self.excluded_queue_flags))
            && (!self.require_surface_support
                || surface_requirment
                    .queue_supports_surface(physical_device, queue_index)
                    .unwrap_or(false))
    }

    pub fn find_queue(
        &self,
        queue_props: &[vk::QueueFamilyProperties],
        physical_device: vk::PhysicalDevice,
        surface_requirment: &VKSurface,
        ignore_excl: bool,
    ) -> Option<u32> {
        for (queue_index, queue_prop) in queue_props.iter().enumerate() {
            if !self.queue_compatible(
                queue_index as u32,
                queue_prop,
                physical_device,
                surface_requirment,
                ignore_excl,
            ) {
                continue;
            }
            return Some(queue_index as u32);
        }
        None
    }
}

impl Default for VKQueueRequest {
    fn default() -> Self {
        Self {
            role_id: 0,
            required_queue_flags: vk::QueueFlags::empty(),
            excluded_queue_flags: vk::QueueFlags::empty(),
            require_surface_support: false,
        }
    }
}

#[derive(Default)]
pub struct VKQueueRequests(pub Vec<VKQueueRequest>);

impl VKQueueRequests {
    pub fn fulfill_requests(
        &self,
        physical_device: vk::PhysicalDevice,
        instance: &Instance,
        surface_requirment: &VKSurface,
    ) -> Option<VKResolvedQueues> {
        let queue_props =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let mut pending_queues: Vec<(VKQueueRequest, Option<VKResolvedQueue>)> =
            self.iter().map(|req| (*req, None)).collect();

        for (req, resolved) in &mut pending_queues {
            if let Some(index) =
                req.find_queue(&queue_props, physical_device, surface_requirment, false)
            {
                *resolved = Some(VKResolvedQueue {
                    role_id: req.role_id,
                    family_index: index,
                })
            }
        }

        for (req, resolved) in &mut pending_queues {
            if resolved.is_some() {
                continue;
            }

            if let Some(index) =
                req.find_queue(&queue_props, physical_device, surface_requirment, true)
            {
                *resolved = Some(VKResolvedQueue {
                    role_id: req.role_id,
                    family_index: index,
                })
            }
        }

        pending_queues
            .into_iter()
            .map(|(_, resolved)| resolved)
            .collect::<Option<VKResolvedQueues>>()
    }
}

impl std::ops::Deref for VKQueueRequests {
    type Target = Vec<VKQueueRequest>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for VKQueueRequests {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Default, Clone, Copy)]
pub struct VKResolvedQueue {
    pub role_id: u32,
    pub family_index: u32,
}

#[derive(Default)]
pub struct VKResolvedQueues(pub Vec<VKResolvedQueue>);

impl VKResolvedQueues {
    /// gets the queue family index for the associated role id
    pub fn get_queue(&self, role_id: u32) -> Option<u32> {
        Some(
            self.iter()
                .find(|queue| queue.role_id == role_id)?
                .family_index,
        )
    }
}

impl std::ops::Deref for VKResolvedQueues {
    type Target = Vec<VKResolvedQueue>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for VKResolvedQueues {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<VKResolvedQueue> for VKResolvedQueues {
    fn from_iter<T: IntoIterator<Item = VKResolvedQueue>>(iter: T) -> Self {
        VKResolvedQueues(iter.into_iter().collect())
    }
}

pub enum VKDeviceCompat {
    Compatable,
    CompatibleWithQueues(VKResolvedQueues),
    Incompatible,
}

impl VKDeviceCompat {
    pub fn as_option(self) -> Option<VKResolvedQueues> {
        match self {
            Self::CompatibleWithQueues(resolve) => Some(resolve),
            _ => None,
        }
    }
}
