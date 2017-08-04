// Copyright 2015 The Gfx-rs Developers.
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

#![deny(missing_docs, missing_copy_implementations)]

//! Resource handles

use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use {buffer, shade, texture, Backend};
use memory::Typed;

/// Untyped buffer handle
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RawBuffer<B: Backend>(Arc<buffer::Raw<B>>);

impl<B: Backend> Deref for RawBuffer<B> {
    type Target = buffer::Raw<B>;
    fn deref(&self) -> &Self::Target { &self.0 }
}

/// Type-safe buffer handle
#[derive(Derivative)]
#[derivative(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Buffer<B: Backend, T>(
    RawBuffer<B>,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    PhantomData<T>
);

impl<B: Backend, T> Typed for Buffer<B, T> {
    type Raw = RawBuffer<B>;
    fn new(handle: RawBuffer<B>) -> Buffer<B, T> {
        Buffer(handle, PhantomData)
    }

    fn raw(&self) -> &RawBuffer<B> { &self.0 }
}

impl<B: Backend, T> Buffer<B, T> {
    /// Get the associated information about the buffer
    pub fn get_info(&self) -> &buffer::Info { self.raw().get_info() }

    /// Get the number of elements in the buffer.
    ///
    /// Fails if `T` is zero-sized.
    pub fn len(&self) -> usize {
        unsafe { self.raw().len::<T>() }
    }
}

/// Shader Handle
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Shader<B: Backend>(Arc<B::Shader>);

/// Program Handle
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Program<B: Backend>(Arc<shade::Program<B>>);

impl<B: Backend> Deref for Program<B> {
    type Target = shade::Program<B>;
    fn deref(&self) -> &Self::Target { &self.0 }
}

/// Raw Pipeline State Handle
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RawPipelineState<B: Backend>(Arc<B::PipelineStateObject>, Program<B>);

/// Raw texture handle
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RawTexture<B: Backend>(Arc<texture::Raw<B>>);

impl<B: Backend> Deref for RawTexture<B> {
    type Target = texture::Raw<B>;
    fn deref(&self) -> &Self::Target { &self.0 }
}

/// Typed texture object
#[derive(Derivative)]
#[derivative(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Texture<B: Backend, S>(
    RawTexture<B>,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    PhantomData<S>
);

impl<B: Backend, S> Typed for Texture<B, S> {
    type Raw = RawTexture<B>;
    fn new(handle: RawTexture<B>) -> Texture<B, S> {
        Texture(handle, PhantomData)
    }

    fn raw(&self) -> &RawTexture<B> { &self.0 }
}

impl<B: Backend, S> Texture<B, S> {
    /// Get texture descriptor
    pub fn get_info(&self) -> &texture::Info { self.raw().get_info() }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum ViewSource<B: Backend> {
    Buffer(RawBuffer<B>),
    Texture(RawTexture<B>),
}

/// Raw Shader Resource View Handle
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RawShaderResourceView<B: Backend>(Arc<B::ShaderResourceView>, ViewSource<B>);

/// Type-safe Shader Resource View Handle
#[derive(Derivative)]
#[derivative(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ShaderResourceView<B: Backend, T>(
    RawShaderResourceView<B>,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    PhantomData<T>
);

impl<B: Backend, T> Typed for ShaderResourceView<B, T> {
    type Raw = RawShaderResourceView<B>;
    fn new(handle: RawShaderResourceView<B>) -> ShaderResourceView<B, T> {
        ShaderResourceView(handle, PhantomData)
    }

    fn raw(&self) -> &RawShaderResourceView<B> { &self.0 }
}

/// Raw Unordered Access View Handle
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RawUnorderedAccessView<B: Backend>(Arc<B::UnorderedAccessView>, ViewSource<B>);

/// Type-safe Unordered Access View Handle
#[derive(Derivative)]
#[derivative(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UnorderedAccessView<B: Backend, T>(
    RawUnorderedAccessView<B>,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    PhantomData<T>
);

impl<B: Backend, T> Typed for UnorderedAccessView<B, T> {
    type Raw = RawUnorderedAccessView<B>;
    fn new(handle: RawUnorderedAccessView<B>) -> UnorderedAccessView<B, T> {
        UnorderedAccessView(handle, PhantomData)
    }

    fn raw(&self) -> &RawUnorderedAccessView<B> { &self.0 }
}

/// Raw RTV
// TODO: Arc it all
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RawRenderTargetView<B: Backend>(Arc<B::RenderTargetView>, RawTexture<B>, texture::Dimensions);

impl<B: Backend> RawRenderTargetView<B> {
    /// Get target dimensions
    pub fn get_dimensions(&self) -> texture::Dimensions { self.2 }

    /// Get the associated texture
    pub fn get_texture(&self) -> &RawTexture<B> { &self.1 }
}

/// Raw DSV
// TODO: Arc it all
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RawDepthStencilView<B: Backend>(Arc<B::DepthStencilView>, RawTexture<B>, texture::Dimensions);

impl<B: Backend> RawDepthStencilView<B> {
    /// Get target dimensions
    pub fn get_dimensions(&self) -> texture::Dimensions { self.2 }

    /// Get the associated texture
    pub fn get_texture(&self) -> &RawTexture<B> { &self.1 }
}

/// Typed RTV
#[derive(Derivative)]
#[derivative(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RenderTargetView<B: Backend, T>(
    RawRenderTargetView<B>,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    PhantomData<T>
);

impl<B: Backend, T> RenderTargetView<B, T> {
    /// Get target dimensions
    pub fn get_dimensions(&self) -> texture::Dimensions { self.raw().get_dimensions() }
}

impl<B: Backend, T> Typed for RenderTargetView<B, T> {
    type Raw = RawRenderTargetView<B>;
    fn new(h: RawRenderTargetView<B>) -> RenderTargetView<B, T> {
        RenderTargetView(h, PhantomData)
    }

    fn raw(&self) -> &RawRenderTargetView<B> { &self.0 }
}

/// Typed DSV
#[derive(Derivative)]
#[derivative(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DepthStencilView<B: Backend, T>(
    RawDepthStencilView<B>,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    PhantomData<T>
);

impl<B: Backend, T> DepthStencilView<B, T> {
    /// Get target dimensions
    pub fn get_dimensions(&self) -> texture::Dimensions {
        self.raw().get_dimensions()
    }
}

impl<B: Backend, T> Typed for DepthStencilView<B, T> {
    type Raw = RawDepthStencilView<B>;
    fn new(h: RawDepthStencilView<B>) -> DepthStencilView<B, T> {
        DepthStencilView(h, PhantomData)
    }

    fn raw(&self) -> &RawDepthStencilView<B> { &self.0 }
}

/// Sampler Handle
// TODO: Arc it all
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Sampler<B: Backend>(Arc<B::Sampler>, texture::SamplerInfo);

impl<B: Backend> Sampler<B> {
    /// Get sampler info
    pub fn get_info(&self) -> &texture::SamplerInfo { &self.1 }
}

/// Fence Handle
#[derive(Clone, Debug)]
pub struct Fence<B: Backend>(Arc<Mutex<B::Fence>>);

/// Semaphore Handle
#[derive(Clone, Debug)]
pub struct Semaphore<B: Backend>(Arc<Mutex<B::Semaphore>>);

/// Stores reference-counted resources used in a command buffer.
/// Seals actual resource names behind the interface, automatically
/// referencing them both by the Factory on resource creation
/// and the Renderer during CommandBuffer population.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct Manager<B: Backend> {
    buffers:       Vec<Arc<buffer::Raw<B>>>,
    shaders:       Vec<Arc<B::Shader>>,
    programs:      Vec<Arc<shade::Program<B>>>,
    psos:          Vec<Arc<B::PipelineStateObject>>,
    textures:      Vec<Arc<texture::Raw<B>>>,
    srvs:          Vec<Arc<B::ShaderResourceView>>,
    uavs:          Vec<Arc<B::UnorderedAccessView>>,
    rtvs:          Vec<Arc<B::RenderTargetView>>,
    dsvs:          Vec<Arc<B::DepthStencilView>>,
    samplers:      Vec<Arc<B::Sampler>>,
    fences:        Vec<Arc<Mutex<B::Fence>>>,
    semaphores:    Vec<Arc<Mutex<B::Semaphore>>>,
}

/// A service trait to be used by the device implementation
#[doc(hidden)]
pub trait Producer<Bd: Backend> {
    fn make_buffer(&mut self,
                   Bd::Buffer,
                   buffer::Info,
                   Option<Bd::Mapping>) -> RawBuffer<Bd>;
    fn make_shader(&mut self, Bd::Shader) -> Shader<Bd>;
    fn make_program(&mut self, Bd::Program, shade::ProgramInfo) -> Program<Bd>;
    fn make_pso(&mut self, Bd::PipelineStateObject, &Program<Bd>) -> RawPipelineState<Bd>;
    fn make_texture(&mut self, Bd::Texture, texture::Info) -> RawTexture<Bd>;
    fn make_buffer_srv(&mut self, Bd::ShaderResourceView, &RawBuffer<Bd>) -> RawShaderResourceView<Bd>;
    fn make_texture_srv(&mut self, Bd::ShaderResourceView, &RawTexture<Bd>) -> RawShaderResourceView<Bd>;
    fn make_buffer_uav(&mut self, Bd::UnorderedAccessView, &RawBuffer<Bd>) -> RawUnorderedAccessView<Bd>;
    fn make_texture_uav(&mut self, Bd::UnorderedAccessView, &RawTexture<Bd>) -> RawUnorderedAccessView<Bd>;
    fn make_rtv(&mut self, Bd::RenderTargetView, &RawTexture<Bd>, texture::Dimensions) -> RawRenderTargetView<Bd>;
    fn make_dsv(&mut self, Bd::DepthStencilView, &RawTexture<Bd>, texture::Dimensions) -> RawDepthStencilView<Bd>;
    fn make_sampler(&mut self, Bd::Sampler, texture::SamplerInfo) -> Sampler<Bd>;
    fn make_fence(&mut self, name: Bd::Fence) -> Fence<Bd>;
    fn make_semaphore(&mut self, Bd::Semaphore) -> Semaphore<Bd>;

    /// Walk through all the handles, keep ones that are reference elsewhere
    /// and call the provided delete function (resource-specific) for others
    fn clean_with<T,
        A: Fn(&mut T, &mut buffer::Raw<Bd>),
        B: Fn(&mut T, &mut Bd::Shader),
        C: Fn(&mut T, &mut shade::Program<Bd>),
        D: Fn(&mut T, &mut Bd::PipelineStateObject),
        E: Fn(&mut T, &mut texture::Raw<Bd>),
        F: Fn(&mut T, &mut Bd::ShaderResourceView),
        G: Fn(&mut T, &mut Bd::UnorderedAccessView),
        H: Fn(&mut T, &mut Bd::RenderTargetView),
        I: Fn(&mut T, &mut Bd::DepthStencilView),
        J: Fn(&mut T, &mut Bd::Sampler),
        K: Fn(&mut T, &mut Mutex<Bd::Fence>),
        L: Fn(&mut T, &mut Mutex<Bd::Semaphore>),
    >(&mut self, &mut T, A, B, C, D, E, F, G, H, I, J, K, L);
}

impl<Bd: Backend> Producer<Bd> for Manager<Bd> {
    fn make_buffer(&mut self,
                   res: Bd::Buffer,
                   info: buffer::Info,
                   mapping: Option<Bd::Mapping>) -> RawBuffer<Bd> {
        let r = Arc::new(buffer::Raw::new(res, info, mapping));
        self.buffers.push(r.clone());
        RawBuffer(r)
    }

    fn make_shader(&mut self, res: Bd::Shader) -> Shader<Bd> {
        let r = Arc::new(res);
        self.shaders.push(r.clone());
        Shader(r)
    }

    fn make_program(&mut self, res: Bd::Program, info: shade::ProgramInfo) -> Program<Bd> {
        let r = Arc::new(shade::Program::new(res, info));
        self.programs.push(r.clone());
        Program(r)
    }

    fn make_pso(&mut self, res: Bd::PipelineStateObject, program: &Program<Bd>) -> RawPipelineState<Bd> {
        let r = Arc::new(res);
        self.psos.push(r.clone());
        RawPipelineState(r, program.clone())
    }

    fn make_texture(&mut self, res: Bd::Texture, info: texture::Info) -> RawTexture<Bd> {
        let r = Arc::new(texture::Raw::new(res, info));
        self.textures.push(r.clone());
        RawTexture(r)
    }

    fn make_buffer_srv(&mut self, res: Bd::ShaderResourceView, buf: &RawBuffer<Bd>) -> RawShaderResourceView<Bd> {
        let r = Arc::new(res);
        self.srvs.push(r.clone());
        RawShaderResourceView(r, ViewSource::Buffer(buf.clone()))
    }

    fn make_texture_srv(&mut self, res: Bd::ShaderResourceView, tex: &RawTexture<Bd>) -> RawShaderResourceView<Bd> {
        let r = Arc::new(res);
        self.srvs.push(r.clone());
        RawShaderResourceView(r, ViewSource::Texture(tex.clone()))
    }

    fn make_buffer_uav(&mut self, res: Bd::UnorderedAccessView, buf: &RawBuffer<Bd>) -> RawUnorderedAccessView<Bd> {
        let r = Arc::new(res);
        self.uavs.push(r.clone());
        RawUnorderedAccessView(r, ViewSource::Buffer(buf.clone()))
    }

    fn make_texture_uav(&mut self, res: Bd::UnorderedAccessView, tex: &RawTexture<Bd>) -> RawUnorderedAccessView<Bd> {
        let r = Arc::new(res);
        self.uavs.push(r.clone());
        RawUnorderedAccessView(r, ViewSource::Texture(tex.clone()))
    }

    fn make_rtv(&mut self, res: Bd::RenderTargetView, tex: &RawTexture<Bd>, dim: texture::Dimensions) -> RawRenderTargetView<Bd> {
        let r = Arc::new(res);
        self.rtvs.push(r.clone());
        RawRenderTargetView(r, tex.clone(), dim)
    }

    fn make_dsv(&mut self, res: Bd::DepthStencilView, tex: &RawTexture<Bd>, dim: texture::Dimensions) -> RawDepthStencilView<Bd> {
        let r = Arc::new(res);
        self.dsvs.push(r.clone());
        RawDepthStencilView(r, tex.clone(), dim)
    }

    fn make_sampler(&mut self, res: Bd::Sampler, info: texture::SamplerInfo) -> Sampler<Bd> {
        let r = Arc::new(res);
        self.samplers.push(r.clone());
        Sampler(r, info)
    }

    fn make_fence(&mut self, res: Bd::Fence) -> Fence<Bd> {
        let r = Arc::new(Mutex::new(res));
        self.fences.push(r.clone());
        Fence(r)
    }

    fn make_semaphore(&mut self, res: Bd::Semaphore) -> Semaphore<Bd> {
        let r = Arc::new(Mutex::new(res));
        self.semaphores.push(r.clone());
        Semaphore(r)
    }

    fn clean_with<T,
        A: Fn(&mut T, &mut buffer::Raw<Bd>),
        B: Fn(&mut T, &mut Bd::Shader),
        C: Fn(&mut T, &mut shade::Program<Bd>),
        D: Fn(&mut T, &mut Bd::PipelineStateObject),
        E: Fn(&mut T, &mut texture::Raw<Bd>),
        F: Fn(&mut T, &mut Bd::ShaderResourceView),
        G: Fn(&mut T, &mut Bd::UnorderedAccessView),
        H: Fn(&mut T, &mut Bd::RenderTargetView),
        I: Fn(&mut T, &mut Bd::DepthStencilView),
        J: Fn(&mut T, &mut Bd::Sampler),
        K: Fn(&mut T, &mut Mutex<Bd::Fence>),
        L: Fn(&mut T, &mut Mutex<Bd::Semaphore>),
    >(&mut self, param: &mut T, fa: A, fb: B, fc: C, fd: D, fe: E, ff: F, fg: G, fh: H, fi: I, fj: J, fk: K, fl: L) {
        fn clean_vec<X, Param, Fun>(param: &mut Param, vector: &mut Vec<Arc<X>>, fun: Fun)
            where Fun: Fn(&mut Param, &mut X)
        {
            let mut temp = Vec::new();
            // delete unique resources and make a list of their indices
            for (i, v) in vector.iter_mut().enumerate() {
                if let Some(x) = Arc::get_mut(v) {
                    fun(param, x);
                    temp.push(i);
                }
            }
            // update the resource vector by removing the elements
            // starting from the last one
            for t in temp.iter().rev() {
                vector.swap_remove(*t);
            }
        }
        clean_vec(param, &mut self.buffers,       fa);
        clean_vec(param, &mut self.shaders,       fb);
        clean_vec(param, &mut self.programs,      fc);
        clean_vec(param, &mut self.psos,          fd);
        clean_vec(param, &mut self.textures,      fe);
        clean_vec(param, &mut self.srvs,          ff);
        clean_vec(param, &mut self.uavs,          fg);
        clean_vec(param, &mut self.rtvs,          fh);
        clean_vec(param, &mut self.dsvs,          fi);
        clean_vec(param, &mut self.samplers,      fj);
        clean_vec(param, &mut self.fences,        fk);
        clean_vec(param, &mut self.semaphores,    fl);
    }
}

impl<B: Backend> Manager<B> {
    /// Create a new handle manager
    pub fn new() -> Manager<B> {
        Manager {
            buffers: Vec::new(),
            shaders: Vec::new(),
            programs: Vec::new(),
            psos: Vec::new(),
            textures: Vec::new(),
            srvs: Vec::new(),
            uavs: Vec::new(),
            rtvs: Vec::new(),
            dsvs: Vec::new(),
            samplers: Vec::new(),
            fences: Vec::new(),
            semaphores: Vec::new(),
        }
    }
    /// Clear all references
    pub fn clear(&mut self) {
        self.buffers.clear();
        self.shaders.clear();
        self.programs.clear();
        self.psos.clear();
        self.textures.clear();
        self.srvs.clear();
        self.uavs.clear();
        self.rtvs.clear();
        self.dsvs.clear();
        self.samplers.clear();
        self.fences.clear();
        self.semaphores.clear();
    }
    /// Extend with all references of another handle manager
    pub fn extend(&mut self, other: &Manager<B>) {
        self.buffers   .extend(other.buffers   .iter().map(|h| h.clone()));
        self.shaders   .extend(other.shaders   .iter().map(|h| h.clone()));
        self.programs  .extend(other.programs  .iter().map(|h| h.clone()));
        self.psos      .extend(other.psos      .iter().map(|h| h.clone()));
        self.textures  .extend(other.textures  .iter().map(|h| h.clone()));
        self.srvs      .extend(other.srvs      .iter().map(|h| h.clone()));
        self.uavs      .extend(other.uavs      .iter().map(|h| h.clone()));
        self.rtvs      .extend(other.rtvs      .iter().map(|h| h.clone()));
        self.dsvs      .extend(other.dsvs      .iter().map(|h| h.clone()));
        self.samplers  .extend(other.samplers  .iter().map(|h| h.clone()));
        self.fences    .extend(other.fences    .iter().map(|h| h.clone()));
        self.semaphores.extend(other.semaphores.iter().map(|h| h.clone()));
    }
    /// Count the total number of referenced resources
    pub fn count(&self) -> usize {
        self.buffers.len() +
        self.shaders.len() +
        self.programs.len() +
        self.psos.len() +
        self.textures.len() +
        self.srvs.len() +
        self.uavs.len() +
        self.rtvs.len() +
        self.dsvs.len() +
        self.samplers.len() +
        self.fences.len() +
        self.semaphores.len()
    }
    /// Reference a buffer
    pub fn ref_buffer<'a>(&mut self, handle: &'a RawBuffer<B>) -> &'a B::Buffer {
        self.buffers.push(handle.0.clone());
        handle.resource()
    }
    /// Reference a shader
    pub fn ref_shader<'a>(&mut self, handle: &'a Shader<B>) -> &'a B::Shader {
        self.shaders.push(handle.0.clone());
        &handle.0
    }
    /// Reference a program
    pub fn ref_program<'a>(&mut self, handle: &'a Program<B>) -> &'a B::Program {
        self.programs.push(handle.0.clone());
        handle.resource()
    }
    /// Reference a pipeline state object
    pub fn ref_pso<'a>(&mut self, handle: &'a RawPipelineState<B>) -> (&'a B::PipelineStateObject, &'a B::Program) {
        self.psos.push(handle.0.clone());
        self.programs.push((handle.1).0.clone());
        (&handle.0, handle.1.resource())
    }
    /// Reference a texture
    pub fn ref_texture<'a>(&mut self, handle: &'a RawTexture<B>) -> &'a B::Texture {
        self.textures.push(handle.0.clone());
        handle.resource()
    }
    /// Reference a shader resource view
    pub fn ref_srv<'a>(&mut self, handle: &'a RawShaderResourceView<B>) -> &'a B::ShaderResourceView {
        self.srvs.push(handle.0.clone());
        &handle.0
    }
    /// Reference an unordered access view
    pub fn ref_uav<'a>(&mut self, handle: &'a RawUnorderedAccessView<B>) -> &'a B::UnorderedAccessView {
        self.uavs.push(handle.0.clone());
        &handle.0
    }
    /// Reference an RTV
    pub fn ref_rtv<'a>(&mut self, handle: &'a RawRenderTargetView<B>) -> &'a B::RenderTargetView {
        self.rtvs.push(handle.0.clone());
        self.textures.push((handle.1).0.clone());
        &handle.0
    }
    /// Reference a DSV
    pub fn ref_dsv<'a>(&mut self, handle: &'a RawDepthStencilView<B>) -> &'a B::DepthStencilView {
        self.dsvs.push(handle.0.clone());
        self.textures.push((handle.1).0.clone());
        &handle.0
    }
    /// Reference a sampler
    pub fn ref_sampler<'a>(&mut self, handle: &'a Sampler<B>) -> &'a B::Sampler {
        self.samplers.push(handle.0.clone());
        &handle.0
    }
    /// Reference a fence
    pub fn ref_fence<'a>(&mut self, fence: &'a Fence<B>) -> &'a Mutex<B::Fence> {
        self.fences.push(fence.0.clone());
        &fence.0
    }
    /// Reference a semaphore
    pub fn ref_semaphore<'a>(&mut self, semaphore: &'a Semaphore<B>) -> &'a Mutex<B::Semaphore> {
        self.semaphores.push(semaphore.0.clone());
        &semaphore.0
    }
}
