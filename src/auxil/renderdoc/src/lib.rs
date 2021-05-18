#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications
)]

//! RenderDoc integration - https://renderdoc.org/

/// The dynamically loaded RenderDoc API function table
#[repr(C)]
#[derive(Debug)]
pub struct RenderDocApi {
    api: renderdoc_sys::RENDERDOC_API_1_4_1,
    lib: libloading::Library,
}

unsafe impl Send for RenderDocApi {}

unsafe impl Sync for RenderDocApi {}

/// RenderDoc API type
#[derive(Debug)]
pub enum RenderDoc {
    /// RenderDoc functionality is available
    Available {
        /// RenderDoc API with function pointers
        api: RenderDocApi,
    },
    /// RenderDoc functionality is _not_ available
    NotAvailable {
        /// A description why renderdoc functionality is not available
        reason: String,
    },
}

impl RenderDoc {
    pub unsafe fn new() -> Self {
        type GetApiFn = unsafe extern "C" fn(version: u32, out: *mut *mut std::ffi::c_void) -> i32;

        #[cfg(windows)]
        let renderdoc_filename = "renderdoc.dll";
        #[cfg(all(unix, not(target_os = "android")))]
        let renderdoc_filename = "librenderdoc.so";
        #[cfg(target_os = "android")]
        let renderdoc_filename = "libVkLayer_GLES_RenderDoc.so";

        let renderdoc_lib = match libloading::Library::new(renderdoc_filename) {
            Ok(lib) => lib,
            Err(e) => {
                return RenderDoc::NotAvailable {
                    reason: format!(
                        "Unable to load renderdoc library '{}': {:?}",
                        renderdoc_filename, e
                    ),
                }
            }
        };

        let get_api: libloading::Symbol<GetApiFn> = match renderdoc_lib.get(b"RENDERDOC_GetAPI\0") {
            Ok(api) => api,
            Err(e) => {
                return RenderDoc::NotAvailable {
                    reason: format!(
                        "Unable to get RENDERDOC_GetAPI from renderdoc library '{}': {:?}",
                        renderdoc_filename, e
                    ),
                }
            }
        };
        let mut obj = std::ptr::null_mut();
        match get_api(10401, &mut obj) {
            1 => RenderDoc::Available {
                api: RenderDocApi {
                    api: *(obj as *mut renderdoc_sys::RENDERDOC_API_1_4_1),
                    lib: renderdoc_lib,
                },
            },
            return_value => RenderDoc::NotAvailable {
                reason: format!(
                    "Unable to get API from renderdoc library '{}': {}",
                    renderdoc_filename, return_value
                ),
            },
        }
    }
}

impl Default for RenderDoc {
    fn default() -> Self {
        if !cfg!(debug_assertions) {
            return RenderDoc::NotAvailable {
                reason: "RenderDoc support is only enabled with 'debug_assertions'".into(),
            };
        }
        unsafe { Self::new() }
    }
}
/// A implementation specific handle
pub type Handle = *mut ::std::os::raw::c_void;

impl RenderDoc {
    /// Start a RenderDoc frame capture
    pub unsafe fn start_frame_capture(&self, device_handle: Handle, window_handle: Handle) {
        match self {
            Self::Available { api: ref entry } => {
                entry.api.StartFrameCapture.unwrap()(device_handle, window_handle);
            }
            Self::NotAvailable { ref reason } => {
                log::warn!("Could not start RenderDoc frame capture: {}", reason)
            }
        };
    }

    /// End a RenderDoc frame capture
    pub unsafe fn end_frame_capture(&self, device_handle: Handle, window_handle: Handle) {
        match self {
            Self::Available { api: ref entry } => {
                entry.api.EndFrameCapture.unwrap()(device_handle, window_handle);
            }
            Self::NotAvailable { ref reason } => {
                log::warn!("Could not end RenderDoc frame capture: {}", reason)
            }
        };
    }
}
