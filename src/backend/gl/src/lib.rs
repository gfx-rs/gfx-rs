// Copyright 2014 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! OpenGL implementation of a device, striving to support OpenGL 2.0 with at
//! least VAOs, but using newer extensions when available.

#![allow(missing_docs)]
#![deny(missing_copy_implementations)]

#[macro_use]
extern crate log;
extern crate gfx_gl as gl;
extern crate gfx_core as core;
extern crate smallvec;

use std::cell::RefCell;
use std::rc::Rc;
use core::{self as c, handle, format, target, texture as t, memory, command as com, buffer, Viewport};
use core::QueueType;
use core::target::{Layer, Level};
use command::{Command, DataBuffer};
use device::MappingKind;
use smallvec::SmallVec;

pub use self::device::Device;
pub use self::info::{Info, PlatformName, Version};

mod command;
mod device;
mod info;
mod native;
mod pool;
mod state;
mod texture;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Backend {}
impl c::Backend for Backend {
    type Adapter = Adapter;
    type CommandQueue = CommandQueue;
    type RawCommandBuffer = command::RawCommandBuffer;
    type SubpassCommandBuffer = command::SubpassCommandBuffer;
    type SubmitInfo = command::SubmitInfo;
    type Device = Device;
    type QueueFamily = QueueFamily;

    type RawCommandPool = pool::RawCommandPool;
    type SubpassCommandPool = pool::SubpassCommandPool;

    type DescriptorPool = pool:: DescriptorPool;

    type Buffer              = native::Buffer;
    type RenderTargetView    = native::TargetView;
    type DepthStencilView    = native::TargetView;
    type ShaderResourceView  = native::ResourceView;
    type UnorderedAccessView = ();
    type Sampler             = native::FatSampler;
    type Fence               = native::Fence;
    type Semaphore           = (); // TODO
    type Mapping             = device::MappingGate;
    type Image               = native::Image;
    type ComputePipeline     = native::PipelineState;
    type GraphicsPipeline    = native::PipelineState;
    type PipelineLayout = ();
    type DescriptorSet       = native::DescriptorSet;
    type ShaderLib = ();
    type DescriptorSetLayout = native::DescriptorSetLayout;
    type RenderPass = ();
    type FrameBuffer = ();
    type DescriptorHeap      = native::DescriptorHeap;
}

/*
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct BufferElement {
    pub desc: c::pso::VertexBufferDesc,
    pub elem: c::pso::Element<format::Format>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct OutputMerger {
    pub draw_mask: u32,
    pub stencil: Option<s::Stencil>,
    pub depth: Option<s::Depth>,
    pub colors: Vec<s::Color>,
}
*/

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Error {
    NoError,
    InvalidEnum,
    InvalidValue,
    InvalidOperation,
    InvalidFramebufferOperation,
    OutOfMemory,
    UnknownError,
}

impl Error {
    pub fn from_error_code(error_code: gl::types::GLenum) -> Error {
        match error_code {
            gl::NO_ERROR                      => Error::NoError,
            gl::INVALID_ENUM                  => Error::InvalidEnum,
            gl::INVALID_VALUE                 => Error::InvalidValue,
            gl::INVALID_OPERATION             => Error::InvalidOperation,
            gl::INVALID_FRAMEBUFFER_OPERATION => Error::InvalidFramebufferOperation,
            gl::OUT_OF_MEMORY                 => Error::OutOfMemory,
            _                                 => Error::UnknownError,
        }
    }
}

type ArrayBuffer = gl::types::GLuint;

/// Create the proxy target views (RTV and DSV) for the attachments of the
/// main framebuffer. These have GL names equal to 0.
/// Not supposed to be used by the users directly.
pub fn create_main_targets_raw(dim: t::Dimensions, color_format: format::SurfaceType, depth_format: format::SurfaceType)
                               -> (handle::RawRenderTargetView<Backend>, handle::RawDepthStencilView<Backend>) {
    use core::handle::Producer;
    let mut temp = handle::Manager::new();
    let color_tex = temp.make_image(
        native::Image::Surface(0),
        t::Info {
            levels: 1,
            kind: t::Kind::D2(dim.0, dim.1, dim.3),
            format: color_format,
            bind: memory::RENDER_TARGET | memory::TRANSFER_SRC,
            usage: memory::Usage::Data,
        },
    );
    let depth_tex = temp.make_image(
        native::Image::Surface(0),
        t::Info {
            levels: 1,
            kind: t::Kind::D2(dim.0, dim.1, dim.3),
            format: depth_format,
            bind: memory::DEPTH_STENCIL | memory::TRANSFER_SRC,
            usage: memory::Usage::Data,
        },
    );
    let m_color = temp.make_rtv(native::TargetView::Surface(0), &color_tex, dim);
    let m_ds = temp.make_dsv(native::TargetView::Surface(0), &depth_tex, dim);
    (m_color, m_ds)
}

/// Internal struct of shared data between the device and its factories.
#[doc(hidden)]
pub struct Share {
    context: gl::Gl,
    info: Info,
    features: c::Features,
    limits: c::Limits,
    private_caps: info::PrivateCaps,
    handles: RefCell<handle::Manager<Backend>>,
}

impl Share {
    /// Fails during a debug build if the implementation's error flag was set.
    pub fn check(&self) -> Result<(), Error> {
        if cfg!(debug_assertions) {
            let gl = &self.context;
            let err = Error::from_error_code(unsafe { gl.GetError() });
            if err != Error::NoError {
                return Err(err)
            }
        }
        Ok(())
    }
}

pub struct Adapter {
    share: Rc<Share>,
    adapter_info: c::AdapterInfo,
    queue_family: [(QueueFamily, QueueType); 1],
}

impl Adapter {
    #[doc(hidden)]
    pub fn new<F>(fn_proc: F) -> Self where
        F: FnMut(&str) -> *const std::os::raw::c_void
    {
        let gl = gl::Gl::load_with(fn_proc);
        // query information
        let (info, features, limits, private_caps) = info::get(&gl);
        info!("Vendor: {:?}", info.platform_name.vendor);
        info!("Renderer: {:?}", info.platform_name.renderer);
        info!("Version: {:?}", info.version);
        info!("Shading Language: {:?}", info.shading_language);
        debug!("Loaded Extensions:");
        for extension in info.extensions.iter() {
            debug!("- {}", *extension);
        }

        let adapter_info = c::AdapterInfo {
            name: info.platform_name.renderer.into(),
            vendor: 0, // TODO
            device: 0, // TODO
            software_rendering: false, // not always true ..
        };

        let queue_type = {
            use info::Requirement::{Core, Es};
            let compute_supported = info.is_supported(&[Core(4,3), Es(3, 1)]); // TODO: extension
            if compute_supported {
                QueueType::General
            } else {
                QueueType::Graphics
            }
        };

        // create the shared context
        let handles = handle::Manager::new();
        let share = Share {
            context: gl,
            info,
            features,
            limits,
            private_caps,
            handles: RefCell::new(handles),
        };

        Adapter {
            share: Rc::new(share),
            adapter_info: adapter_info,
            queue_family: [(QueueFamily, queue_type)],
        }
    }
}

impl c::Adapter<Backend> for Adapter {
    fn open(&self, queue_descs: &[(&QueueFamily, QueueType, u32)]) -> c::Gpu<Backend> {
        // Only support a single queue
        assert_eq!(queue_descs.len(), 1);
        assert!(queue_descs[0].2 <= 1);

        // initialize permanent states
        let gl = &self.share.context;
        if self.share.features.srgb_color {
            unsafe {
                gl.Enable(gl::FRAMEBUFFER_SRGB);
            }
        }
        unsafe {
            gl.PixelStorei(gl::UNPACK_ALIGNMENT, 1);

            if !self.share.info.version.is_embedded {
                gl.Enable(gl::PROGRAM_POINT_SIZE);
            }
        }

        // create main VAO and bind it
        let mut vao = 0;
        if self.share.private_caps.array_buffer_supported {
            unsafe {
                gl.GenVertexArrays(1, &mut vao);
                gl.BindVertexArray(vao);
            }
        }

        let mut gpu = c::Gpu {
            device: Device::new(self.share.clone()),
            general_queues: Vec::new(),
            graphics_queues: Vec::new(),
            compute_queues: Vec::new(),
            transfer_queues: Vec::new(),
            heap_types: Vec::new(), // TODO
            memory_heaps: Vec::new(), // TODO
        };

        let raw_queue = || {
            CommandQueue {
                share: self.share.clone(),
                vao,
                state: State::new(),
                frame_handles: handle::Manager::new(),
                max_resource_count: Some(999999),
            }
        };

        let (_, queue_type, num_queues) = queue_descs[0];
        for _ in 0..num_queues {
            unsafe {
                match queue_type {
                    QueueType::General => {
                        gpu.general_queues.push(c::GeneralQueue::new(raw_queue()));
                    }
                    QueueType::Graphics => {
                        gpu.graphics_queues.push(c::GraphicsQueue::new(raw_queue()));
                    }
                    QueueType::Compute => {
                        gpu.compute_queues.push(c::ComputeQueue::new(raw_queue()));
                    }
                    QueueType::Transfer => {
                        gpu.transfer_queues.push(c::TransferQueue::new(raw_queue()));
                    }
                }
            }
        }

        gpu
    }

    fn get_info(&self) -> &c::AdapterInfo {
        &self.adapter_info
    }

    fn get_queue_families(&self) -> &[(QueueFamily, QueueType)] {
        &self.queue_family
    }
}

pub struct CommandQueue {
    share: Rc<Share>,
    vao: ArrayBuffer,
    state: State,
    frame_handles: handle::Manager<Backend>,
    max_resource_count: Option<usize>,
}

// State caching system for command queue.
//
// We track the current global state, which is based on
// the restriction that we only expose _one_ command queue.
//
// This allows us to minimize additional driver calls to
// ensure that command buffers are handled isolated of each other.
struct State {
    // Indicate if the vertex array object is bound.
    // If VAOs are not supported, this will be also set to true.
    vao: bool,
    // Currently bound index/element buffer.
    // None denotes that we don't know what is currently bound.
    index_buffer: Option<gl::types::GLuint>,
    // Currently set viewports.
    num_viewports: usize,
    // Currently set scissor rects.
    num_scissors: usize,
}

impl State {
    // Create a new state, respresenting the initial context state
    // as exposed by OpenGL.
    fn new() -> Self {
        State {
            vao: false,
            index_buffer: None,
            num_viewports: 0,
            num_scissors: 0,
        }
    }

    // Invalidate the current state, forcing a complete reset.
    // Required if we allow users to manually inject OpenGL calls.
    fn flush(&mut self) {
        self.vao = false;
        self.index_buffer = None;

        // TOOD: reset viewports and scissors
        //       do we need to clear everything from 0..MAX_VIEWPORTS?
    }
}

impl CommandQueue {
    /// Access the OpenGL directly via a closure. OpenGL types and enumerations
    /// can be found in the `gl` crate.
    ///
    /// > Note: Calling this function can have a noticeable impact on the performance
    ///         because the internal state cache will flushed.
    pub unsafe fn with_gl<F: FnMut(&gl::Gl)>(&mut self, mut fun: F) {
        self.reset_state();
        fun(&self.share.context);
        // Flush the state to enforce a reset once a new command buffer
        // is execute because we have no control of the called functions.
        self.state.flush();
    }

    /*
    fn bind_attribute(&mut self, slot: c::AttributeSlot, buffer: n::Buffer, bel: BufferElement) {
        use core::format::SurfaceType as S;
        use core::format::ChannelType as C;
        let (fm8, fm16, fm32) = match bel.elem.format.1 {
            C::Int | C::Inorm =>
                (gl::BYTE, gl::SHORT, gl::INT),
            C::Uint | C::Unorm =>
                (gl::UNSIGNED_BYTE, gl::UNSIGNED_SHORT, gl::UNSIGNED_INT),
            C::Float => (gl::ZERO, gl::HALF_FLOAT, gl::FLOAT),
            C::Srgb => {
                error!("Unsupported Srgb channel type");
                return
            }
        };
        let (count, gl_type) = match bel.elem.format.0 {
            S::R8              => (1, fm8),
            S::R8_G8           => (2, fm8),
            S::R8_G8_B8_A8     => (4, fm8),
            S::R16             => (1, fm16),
            S::R16_G16         => (2, fm16),
            S::R16_G16_B16     => (3, fm16),
            S::R16_G16_B16_A16 => (4, fm16),
            S::R32             => (1, fm32),
            S::R32_G32         => (2, fm32),
            S::R32_G32_B32     => (3, fm32),
            S::R32_G32_B32_A32 => (4, fm32),
            _ => {
                error!("Unsupported element type: {:?}", bel.elem.format.0);
                return
            }
        };
        let gl = &self.share.context;
        unsafe { gl.BindBuffer(gl::ARRAY_BUFFER, buffer) };
        let offset = bel.elem.offset as *const gl::types::GLvoid;
        let stride = bel.desc.stride as gl::types::GLint;
        match bel.elem.format.1 {
            C::Int | C::Uint => unsafe {
                gl.VertexAttribIPointer(slot as gl::types::GLuint,
                    count, gl_type, stride, offset);
            },
            C::Inorm | C::Unorm => unsafe {
                gl.VertexAttribPointer(slot as gl::types::GLuint,
                    count, gl_type, gl::TRUE, stride, offset);
            },
            //C::Iscaled | C::Uscaled => unsafe {
            //    gl.VertexAttribPointer(slot as gl::types::GLuint,
            //        count, gl_type, gl::FALSE, stride, offset);
            //},
            C::Float => unsafe {
                gl.VertexAttribPointer(slot as gl::types::GLuint,
                    count, gl_type, gl::FALSE, stride, offset);
            },
            C::Srgb => (),
        }
        unsafe { gl.EnableVertexAttribArray(slot as gl::types::GLuint) };
        if self.share.capabilities.instance_rate_supported {
            unsafe { gl.VertexAttribDivisor(slot as gl::types::GLuint,
                bel.desc.rate as gl::types::GLuint) };
        } else if bel.desc.rate != 0 {
            error!("Instanced arrays are not supported");
        }
    }
    */

    fn bind_target(&mut self, point: gl::types::GLenum, attachment: gl::types::GLenum, view: &native::TargetView) {
        let gl = &self.share.context;
        match view {
            &native::TargetView::Surface(surface) => unsafe {
                gl.FramebufferRenderbuffer(point, attachment, gl::RENDERBUFFER, surface);
            },
            &native::TargetView::Texture(texture, level) => unsafe {
                gl.FramebufferTexture(point, attachment, texture,
                                      level as gl::types::GLint);
            },
            &native::TargetView::TextureLayer(texture, level, layer) => unsafe {
                gl.FramebufferTextureLayer(point, attachment, texture,
                                           level as gl::types::GLint,
                                           layer as gl::types::GLint);
            },
        }
    }

    fn unbind_target(&mut self, point: gl::types::GLenum, attachment: gl::types::GLenum) {
        let gl = &self.share.context;
        unsafe { gl.FramebufferTexture(point, attachment, 0, 0) };
    }

    // Reset the state to match our _expected_ state before executing
    // a command buffer.
    fn reset_state(&mut self) {
        let gl = &self.share.context;
        let priv_caps = &self.share.private_caps;

        // Bind default VAO
        if !self.state.vao {
            if priv_caps.array_buffer_supported {
                unsafe { gl.BindVertexArray(self.vao) };
            }
            self.state.vao = true
        }

        // Unbind index buffers
        match self.state.index_buffer {
            Some(0) => (), // Nothing to do
            Some(_) | None => {
                unsafe { gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0) };
                self.state.index_buffer = Some(0);
            }
        }

        // Reset viewports
        if self.state.num_viewports == 1 {
            unsafe { gl.Viewport(0, 0, 0, 0) };
            unsafe { gl.DepthRange(0.0, 1.0) };
        } else if self.state.num_viewports > 1 {
            // 16 viewports is a common limit set in drivers.
            let viewports: SmallVec<[[f32; 4]; 16]> =
                (0..self.state.num_viewports)
                    .map(|_| [0.0, 0.0, 0.0, 0.0])
                    .collect();
            let depth_ranges: SmallVec<[[f64; 2]; 16]> =
                (0..self.state.num_viewports)
                    .map(|_| [0.0, 0.0])
                    .collect();
            unsafe { gl.ViewportArrayv(0, viewports.len() as i32, viewports.as_ptr() as *const _)};
            unsafe { gl.DepthRangeArrayv(0, depth_ranges.len() as i32, depth_ranges.as_ptr() as *const _)};
        }

        // Reset scissors
        if self.state.num_scissors == 1 {
            unsafe { gl.Scissor(0, 0, 0, 0) };
        } else if self.state.num_scissors > 1 {
            // 16 viewports is a common limit set in drivers.
            let scissors: SmallVec<[[i32; 4]; 16]> =
                (0..self.state.num_scissors)
                    .map(|_| [0, 0, 0, 0])
                    .collect();
            unsafe { gl.ScissorArrayv(0, scissors.len() as i32, scissors.as_ptr() as *const _)};
        }
    }

    fn process(&mut self, cmd: &Command, data_buf: &DataBuffer) {
        match *cmd {
            Command::BindIndexBuffer(buffer) => {
                let gl = &self.share.context;
                self.state.index_buffer = Some(buffer);
                unsafe { gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, buffer) };
            }
            Command::BindVertexBuffers(data_ptr) => {
                unimplemented!()
            }
            Command::Draw { primitive, start, count, instances } => {
                let gl = &self.share.context;
                let features = &self.share.features;
                match instances {
                    Some((num, base)) => unsafe {
                        if features.draw_instanced {
                            if features.draw_instanced_base {
                                gl.DrawArraysInstancedBaseInstance(
                                    primitive,
                                    start as gl::types::GLsizei,
                                    count as gl::types::GLsizei,
                                    num as gl::types::GLsizei,
                                    base as gl::types::GLuint,
                                );
                            } else if base == 0 {
                                gl.DrawArraysInstanced(
                                    primitive,
                                    start as gl::types::GLsizei,
                                    count as gl::types::GLsizei,
                                    num as gl::types::GLsizei,
                                );
                            } else {
                                error!("Instanced draw calls with non-zero base instance are not supported");
                            }
                        } else {
                            error!("Instanced draw calls are not supported");
                        }
                    },
                    None => unsafe {
                        gl.DrawArrays(
                            primitive,
                            start as gl::types::GLsizei,
                            count as gl::types::GLsizei
                        );
                    },
                }
            }
            Command::DrawIndexed { primitive, index_type, start, count, base: base_vertex, instances } => {
                let gl = &self.share.context;
                let features = &self.share.features;
                let offset = start as *const gl::types::GLvoid;

                match instances {
                    Some((num, base_instance)) => unsafe {
                        if features.draw_indexed_instanced_base {
                            // fully compatible
                            gl.DrawElementsInstancedBaseVertexBaseInstance(
                                primitive,
                                count as gl::types::GLsizei,
                                index_type,
                                offset,
                                num as gl::types::GLsizei,
                                base_vertex as gl::types::GLint,
                                base_instance as gl::types::GLuint,
                            );
                        } else if base_instance != 0 {
                            error!("Instance bases with instanced indexed drawing is not supported")
                        } else {
                            // No base instance
                            if features.draw_indexed_instanced_base_vertex {
                                gl.DrawElementsInstancedBaseVertex(
                                    primitive,
                                    count as gl::types::GLsizei,
                                    index_type,
                                    offset,
                                    num as gl::types::GLsizei,
                                    base_vertex as gl::types::GLint,
                                );
                            } else if base_vertex != 0 {
                                error!("Base vertex with instanced indexed drawing is not supported")
                            } else {
                                // No base instance and base vertex
                                if features.draw_indexed_instanced {
                                    gl.DrawElementsInstanced(
                                        primitive,
                                        count as gl::types::GLsizei,
                                        index_type,
                                        offset,
                                        num as gl::types::GLsizei,
                                    );
                                } else {
                                    error!("Instanced indexed drawing is not supported")
                                }
                            }
                        }
                    },
                    None => unsafe {
                        if features.draw_indexed_base {
                            gl.DrawElementsBaseVertex(
                                primitive,
                                count as gl::types::GLsizei,
                                index_type,
                                offset,
                                base_vertex as gl::types::GLint,
                            );
                        } else if base_vertex == 0 {
                            gl.DrawElements(
                                primitive,
                                count as gl::types::GLsizei,
                                index_type,
                                offset,
                            );
                        } else {
                            error!("Base vertex with indexed drawing not supported");
                        }
                    },
                }
            }
            Command::Dispatch(x, y, z) => {
                // Capability support is given by which queue types will be exposed.
                // If there is no compute support, this pattern should never be reached
                // because no queue with compute capability can be created.
                let gl = &self.share.context;
                unsafe { gl.DispatchCompute(x, y, z) };
            }
            Command::DispatchIndirect(buffer, offset) => {
                // Capability support is given by which queue types will be exposed.
                // If there is no compute support, this pattern should never be reached
                // because no queue with compute capability can be created.
                let gl = &self.share.context;
                unsafe {
                    gl.BindBuffer(gl::DRAW_INDIRECT_BUFFER, buffer);
                    // TODO: possible integer conversion issue
                    gl.DispatchComputeIndirect(offset as gl::types::GLintptr);
                }
            }
            Command::SetViewports { viewport_ptr, depth_range_ptr } => {
                let gl = &self.share.context;
                let viewports = data_buf.get::<[f32; 4]>(viewport_ptr);
                let depth_ranges = data_buf.get::<[f64; 2]>(depth_range_ptr);

                let num_viewports = viewports.len();
                assert_eq!(num_viewports, depth_ranges.len());
                assert!(0 < num_viewports && num_viewports <= self.share.limits.max_viewports);

                if num_viewports == 1 {
                    let view = viewports[0];
                    let depth_range  = depth_ranges[0];
                    unsafe { gl.Viewport(view[0] as i32, view[1] as i32, view[2] as i32, view[3] as i32) };
                    unsafe { gl.DepthRange(depth_range[0], depth_range[1]) };
                } else if num_viewports > 1 {
                    // Support for these functions is coupled with the support
                    // of multiple viewports.
                    unsafe { gl.ViewportArrayv(0, num_viewports as i32, viewports.as_ptr() as *const _) };
                    unsafe { gl.DepthRangeArrayv(0, num_viewports as i32, depth_ranges.as_ptr() as *const _) };
                }
            }
            Command::SetScissors(data_ptr) => {
                let gl = &self.share.context;
                let scissors = data_buf.get::<[i32; 4]>(data_ptr);
                let num_scissors = scissors.len();
                assert!(0 < num_scissors && num_scissors <= self.share.limits.max_viewports);

                if num_scissors == 1 {
                    let scissor = scissors[0];
                    unsafe { gl.Scissor(scissor[0], scissor[1], scissor[2], scissor[3]) };
                } else {
                    // Support for this function is coupled with the support
                    // of multiple viewports.
                    unsafe { gl.ScissorArrayv(0, num_scissors as i32, scissors.as_ptr() as *const _) };
                }
            }

            /*
            Command::Clear(color, depth, stencil) => {
                let gl = &self.share.context;
                if self.share.private_caps.clear_buffer_supported {
                    if let Some(c) = color {
                        let slot = 0; //TODO?
                        state::unlock_color_mask(gl);
                        match c {
                            com::ClearColor::Float(v) => unsafe {
                                gl.ClearBufferfv(gl::COLOR, slot, &v[0]);
                            },
                            com::ClearColor::Int(v) => unsafe {
                                gl.ClearBufferiv(gl::COLOR, slot, &v[0]);
                            },
                            com::ClearColor::Uint(v) => unsafe {
                                gl.ClearBufferuiv(gl::COLOR, slot, &v[0]);
                            },
                        }
                    }
                    if let Some(ref d) = depth {
                        unsafe {
                            gl.DepthMask(gl::TRUE);
                            gl.ClearBufferfv(gl::DEPTH, 0, d);
                        }
                    }
                    if let Some(s) = stencil {
                        let v = s as gl::types::GLint;
                        unsafe {
                            gl.StencilMask(gl::types::GLuint::max_value());
                            gl.ClearBufferiv(gl::STENCIL, 0, &v);
                        }
                    }
                } else {
                    let mut flags = 0;
                    if let Some(col) = color {
                        flags |= gl::COLOR_BUFFER_BIT;
                        let v = if let com::ClearColor::Float(v) = col {
                            v
                        } else {
                            warn!("Integer clears are not supported on GL2");
                            [0.0, 0.0, 0.0, 0.0]
                        };
                        state::unlock_color_mask(gl);
                        unsafe {
                            gl.ClearColor(v[0], v[1], v[2], v[3]);
                        }
                    }
                    if let Some(d) = depth {
                        flags |= gl::DEPTH_BUFFER_BIT;
                        unsafe  {
                            gl.DepthMask(gl::TRUE);
                            gl.ClearDepth(d as gl::types::GLdouble);
                        }
                    }
                    if let Some(s) = stencil {
                        flags |= gl::STENCIL_BUFFER_BIT;
                        unsafe  {
                            gl.StencilMask(gl::types::GLuint::max_value());
                            gl.ClearStencil(s as gl::types::GLint);
                        }
                    }
                    unsafe {
                        gl.Clear(flags);
                    }
                }
            },
            Command::BindProgram(program) => unsafe {
                self.share.context.UseProgram(program);
            },
            Command::BindConstantBuffer(pso::ConstantBufferParam(buffer, _, slot)) => unsafe {
                self.share.context.BindBufferBase(gl::UNIFORM_BUFFER, slot as gl::types::GLuint, buffer);
            },
            Command::BindResourceView(pso::ResourceViewParam(view, _, slot)) => unsafe {
                self.share.context.ActiveTexture(gl::TEXTURE0 + slot as gl::types::GLenum);
                self.share.context.BindTexture(view.bind, view.object);
            },
            Command::BindUnorderedView(_uav) => unimplemented!(),
            Command::BindSampler(pso::SamplerParam(sampler, _, slot), bind_opt) => {
                let gl = &self.share.context;
                if self.share.private_caps.sampler_objects_supported {
                    unsafe { gl.BindSampler(slot as gl::types::GLuint, sampler.object) };
                } else {
                    assert!(c::MAX_SAMPLERS <= c::MAX_RESOURCE_VIEWS);
                    debug_assert_eq!(sampler.object, 0);
                    if let Some(bind) = bind_opt {
                        tex::bind_sampler(gl, bind, &sampler.info, &self.share.private_caps);
                    }else {
                        error!("Trying to bind a sampler to slot {}, when sampler objects are not supported, and no texture is bound there", slot);
                    }
                }
            },
            Command::BindPixelTargets(pts) => {
                let point = gl::DRAW_FRAMEBUFFER;
                for i in 0 .. c::MAX_COLOR_TARGETS {
                    let att = gl::COLOR_ATTACHMENT0 + i as gl::types::GLuint;
                    if let Some(ref target) = pts.colors[i] {
                        self.bind_target(point, att, target);
                    } else {
                        self.unbind_target(point, att);
                    }
                }
                if let Some(ref depth) = pts.depth {
                    self.bind_target(point, gl::DEPTH_ATTACHMENT, depth);
                }
                if let Some(ref stencil) = pts.stencil {
                    self.bind_target(point, gl::STENCIL_ATTACHMENT, stencil);
                }
            },
            Command::BindAttribute(slot, buffer,  bel) => {
                self.bind_attribute(slot, buffer, bel);
            },
            Command::UnbindAttribute(slot) => unsafe {
                self.share.context.DisableVertexAttribArray(slot as gl::types::GLuint);
            },
            Command::BindFrameBuffer(point, frame_buffer) => {
                if self.share.private_caps.frame_buffer_supported {
                    let gl = &self.share.context;
                    unsafe { gl.BindFramebuffer(point, frame_buffer) };
                } else if frame_buffer != 0 {
                    error!("Tried to bind FBO {} without FBO support!", frame_buffer);
                }
            },
            Command::BindUniform(loc, uniform) => {
                let gl = &self.share.context;
                shade::bind_uniform(gl, loc as gl::types::GLint, uniform);
            },
            Command::SetDrawColorBuffers(num) => {
                let mask = (1 << (num as usize)) - 1;
                state::bind_draw_color_buffers(&self.share.context, mask);
            },
            Command::SetRasterizer(rast) => {
                state::bind_rasterizer(&self.share.context, &rast, self.share.info.version.is_embedded);
            },
            Command::SetDepthState(depth) => {
                state::bind_depth(&self.share.context, &depth);
            },
            Command::SetStencilState(stencil, refs, cull) => {
                state::bind_stencil(&self.share.context, &stencil, refs, cull);
            },
            Command::SetBlendState(slot, color) => {
                if self.share.capabilities.separate_blending_slots_supported {
                    state::bind_blend_slot(&self.share.context, slot, color);
                }else if slot == 0 {
                    //self.temp.color = color; //TODO
                    state::bind_blend(&self.share.context, color);
                }else if false {
                    error!("Separate blending slots are not supported");
                }
            },
            Command::SetBlendColor(color) => {
                state::set_blend_color(&self.share.context, color);
            },
            Command::SetPatches(num) => {
                let gl = &self.share.context;
                unsafe {
                    gl.PatchParameteri(gl::PATCH_VERTICES, num as gl::types::GLint);
                }
            },
            Command::CopyBuffer(src, dst, src_offset, dst_offset, size) => {
                let gl = &self.share.context;

                if self.share.capabilities.copy_buffer_supported {
                    unsafe {
                        gl.BindBuffer(gl::COPY_READ_BUFFER, src);
                        gl.BindBuffer(gl::COPY_WRITE_BUFFER, dst);
                        gl.CopyBufferSubData(gl::COPY_READ_BUFFER,
                                            gl::COPY_WRITE_BUFFER,
                                            src_offset,
                                            dst_offset,
                                            size);
                    }
                } else {
                    debug_assert!(self.share.private_caps.buffer_storage_supported == false);

                    unsafe {
                        let mut src_ptr = 0 as *mut ::std::os::raw::c_void;
                        device::temporary_ensure_mapped(&mut src_ptr, gl::COPY_READ_BUFFER, src, memory::READ, gl);
                        src_ptr.offset(src_offset);

                        let mut dst_ptr = 0 as *mut ::std::os::raw::c_void;
                        device::temporary_ensure_mapped(&mut dst_ptr, gl::COPY_WRITE_BUFFER, dst, memory::WRITE, gl);
                        dst_ptr.offset(dst_offset);

                        ::std::ptr::copy(src_ptr, dst_ptr, size as usize);

                        device::temporary_ensure_unmapped(&mut src_ptr, gl::COPY_READ_BUFFER, src, gl);
                        device::temporary_ensure_unmapped(&mut dst_ptr, gl::COPY_WRITE_BUFFER, dst, gl);
                    }
                }
            },
            Command::CopyBufferToTexture(src, src_offset, dst, kind, face, img) => {
                match tex::copy_from_buffer(&self.share.context, dst, kind, face, &img, src, src_offset) {
                    Ok(_) => (),
                    Err(e) => error!("GL: {:?} failed: {:?}", cmd, e)
                }
            },
            Command::CopyTextureToBuffer(src, kind, face, img, dst, dst_offset) => {
                match tex::copy_to_buffer(&self.share.context, src, kind, face, &img, dst, dst_offset) {
                    Ok(_) => (),
                    Err(e) => error!("GL: {:?} failed: {:?}", cmd, e)
                }
            },

            */
        }
        if let Err(err) = self.share.check() {
            panic!("Error {:?} executing command: {:?}", err, cmd)
        }
    }

    fn before_submit<'a>(&mut self, gpu_access: &'a com::AccessInfo<Backend>)
                         -> c::SubmissionResult<com::AccessGuard<'a, Backend>> {
        let mut gpu_access = try!(gpu_access.take_accesses());
        if self.share.private_caps.buffer_storage_supported {
            // MappingKind::Persistent
            self.ensure_mappings_flushed(&mut gpu_access);
        } else {
            // MappingKind::Temporary
            self.ensure_mappings_unmapped(&mut gpu_access);
        }
        Ok(gpu_access)
    }

    // MappingKind::Persistent
    fn ensure_mappings_flushed(&mut self, gpu_access: &mut com::AccessGuard<Backend>) {
        let gl = &self.share.context;
        for (buffer, mapping) in gpu_access.access_mapped_reads() {
            let target = device::role_to_target(buffer.get_info().role);
            let status = match &mut mapping.kind {
                &mut MappingKind::Persistent(ref mut status) => status,
                _ => unreachable!(),
            };

            status.ensure_flushed(|| unsafe {
                gl.BindBuffer(target, *buffer.resource());
                let size = buffer.get_info().size as isize;
                gl.FlushMappedBufferRange(target, 0, size);
            });
        }
    }

    // MappingKind::Temporary
    fn ensure_mappings_unmapped(&mut self, gpu_access: &mut com::AccessGuard<Backend>) {
        for (buffer, mapping) in gpu_access.access_mapped() {
            let target = device::role_to_target(buffer.get_info().role);
            device::temporary_ensure_unmapped(&mut mapping.pointer,
                                               target,
                                               *buffer.resource(),
                                               &self.share.context);
        }
    }

    fn after_submit(&mut self, gpu_access: &mut com::AccessGuard<Backend>) {
        if self.share.private_caps.buffer_storage_supported {
            // MappingKind::Persistent
            if gpu_access.has_mapped_reads() || gpu_access.has_mapped_writes() {
                if gpu_access.has_mapped_writes() {
                    self.place_memory_barrier();
                }

                let fence = self.place_fence(); // TODO: do we need a fence here?
                self.track_mapped_gpu_access(gpu_access, &fence);
            }
        }
    }

    fn place_memory_barrier(&mut self) {
        let gl = &self.share.context;
        // TODO: other flags ?
        unsafe { gl.MemoryBarrier(gl::CLIENT_MAPPED_BUFFER_BARRIER_BIT); }
    }

    fn place_fence(&mut self) -> handle::Fence<Backend> {
        use core::handle::Producer;

        let gl = &self.share.context;
        let fence = unsafe {
            gl.FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0)
        };
        self.frame_handles.make_fence(native::Fence(fence))
    }

    fn signal_fence(&mut self, fence: &handle::Fence<Backend>) {
        if self.share.private_caps.sync_supported {
            let gl = &self.share.context;
            let sync = unsafe {
                gl.FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0)
            };

            self.frame_handles.ref_fence(&fence).lock().unwrap().0 = sync;
        }
    }

    // MappingKind::Persistent
    fn track_mapped_gpu_access(&mut self,
                               gpu_access: &mut com::AccessGuard<Backend>,
                               fence: &handle::Fence<Backend>) {
        for (_, mapping) in gpu_access.access_mapped() {
            let status = match &mut mapping.kind {
                &mut MappingKind::Persistent(ref mut status) => status,
                _ => unreachable!(),
            };
            status.gpu_access(fence.clone());
        }
    }

    fn wait_fence(&mut self, fence: &handle::Fence<Backend>) {
        device::wait_fence(
            &*self.frame_handles.ref_fence(&fence).lock().unwrap(),
            &self.share.context,
            1_000_000);
    }
}

impl c::CommandQueue<Backend> for CommandQueue {
    unsafe fn submit_raw<'a, I>(
        &mut self,
        submit_infos: I,
        fence: Option<&handle::Fence<Backend>>,
        access: &com::AccessInfo<Backend>,
    ) where I: Iterator<Item=c::RawSubmission<'a, Backend>> {
        let mut access = self.before_submit(access).unwrap();
        for submit in submit_infos {
            for cb in submit.cmd_buffers {
                self.reset_state();
                for com in &*cb.buf {
                    self.process(com, &*cb.data);
                }
            }
        }
        self.after_submit(&mut access);
        fence.map(|fence| self.signal_fence(fence));
    }

    fn pin_submitted_resources(&mut self, man: &handle::Manager<Backend>) {
        self.frame_handles.extend(man);
        match self.max_resource_count {
            Some(c) if self.frame_handles.count() > c => {
                error!("Way too many resources in the current frame. Did you call Device::cleanup()?");
                self.max_resource_count = None;
            },
            _ => (),
        }
    }

    fn cleanup(&mut self) {
        use core::handle::Producer;
        self.frame_handles.clear();
        self.share.handles.borrow_mut().clean_with(&mut &self.share.context,
            |gl, buffer| {
                buffer.mapping().map(|raw| {
                    // we have exclusive access because it's the last reference
                    let mapping = unsafe { raw.use_access() };
                    match mapping.kind {
                        MappingKind::Persistent(_) => (),
                        MappingKind::Temporary => {
                            let target = device::role_to_target(buffer.get_info().role);
                            device::temporary_ensure_unmapped(&mut mapping.pointer,
                                                               target,
                                                               *buffer.resource(),
                                                               gl);
                        }
                    }
                });
                unsafe { gl.DeleteBuffers(1, buffer.resource()) }
            },
            |gl, pipeline| {}, // TODO: graphics pipeline
            |gl, pipeline| {}, // TODO: compute pipeline
            |gl, renderpass| {}, // TODO: render pass
            /*
            |gl, v| unsafe { gl.DeleteShader(*v) },
            |gl, program| unsafe { gl.DeleteProgram(*program.resource()) },
            |_, _| {}, //PSO
            */
            |gl, raw_texture| match raw_texture.resource() {
                &native::Image::Surface(ref suf) => unsafe { gl.DeleteRenderbuffers(1, suf) },
                &native::Image::Texture(ref tex) => unsafe { gl.DeleteTextures(1, tex) },
            }, // new texture
            |gl, v| if v.owned {
                unsafe { gl.DeleteTextures(1, &v.object) }
            }, //SRV
            |_, _| {}, //UAV
            |_, _| {}, //RTV
            |_, _| {}, //DSV
            |gl, v| unsafe { if v.object != 0 { gl.DeleteSamplers(1, &v.object) }},
            |gl, fence| { fence.get_mut().map(|fence| unsafe { gl.DeleteSync(fence.0) }); },
            |_, _| {}, // Semaphore
            |gl, layout| {}, // TODO: descriptor set layout
            |gl, layout| {}, // TODO: pipeline layout
            |gl, heap| {}, // TODO: descriptor heap
        );
    }
}

#[allow(missing_copy_implementations)]
pub struct QueueFamily;

impl c::QueueFamily for QueueFamily {
    fn num_queues(&self) -> u32 { 1 }
}
