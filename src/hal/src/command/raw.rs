use std::borrow::Borrow;
use std::ops::Range;

use pso;
use {Backend, IndexCount, InstanceCount, VertexCount, VertexOffset};
use buffer::IndexBufferView;
use image::{ImageLayout, SubresourceRange};
use memory::Barrier;
use query::{Query, QueryControl, QueryId};
use super::{
    ColorValue, StencilValue, Rect, Viewport,
    AttachmentClear, BufferCopy, BufferImageCopy,
    ClearColor, ClearDepthStencil, ClearValue,
    ImageCopy, ImageResolve, SubpassContents,
};

/// Unsafe variant of `ClearColor`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union ClearColorRaw {
    /// `f32` variant
    pub float32: [f32; 4],
    /// `i32` variant
    pub int32: [i32; 4],
    /// `u32` variant
    pub uint32: [u32; 4],
    _align: [u32; 4],
}

/// A variant of `ClearDepthStencil` that has a `#[repr(C)]` layout
/// and so is used when a known layout is needed.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ClearDepthStencilRaw {
    /// Depth value
    pub depth: f32,
    /// Stencil value
    pub stencil: u32,
}

/// Unsafe variant of `ClearValue`.
#[repr(C)]
#[derive(Clone, Copy)]
pub union ClearValueRaw {
    /// Clear color
    pub color: ClearColorRaw,
    /// Clear depth and stencil
    pub depth_stencil: ClearDepthStencilRaw,
    _align: [u32; 4],
}

bitflags! {
    /// DOC TODO
    #[derive(Default)]
    pub struct CommandBufferFlags: u16 {
        // TODO: Remove once 'const fn' is stabilized: https://github.com/rust-lang/rust/issues/24111
        ///
        const EMPTY = 0x0;

        ///
        const ONE_TIME_SUBMIT = 0x1;

        ///
        const RENDER_PASS_CONTINUE = 0x2;

        ///
        const SIMULTANEOUS_USE = 0x4;
    }
}

/// An enum that indicates at runtime whether a command buffer
/// is primary or secondary, similar to what `command::Primary`
/// and `command::Secondary` do at compile-time.
/// 
/// DOC TODO: Is this correct?
#[derive(Clone, Copy)]
pub enum Level {
    ///
    Primary,
    ///
    Secondary,
}

/// A trait that describes all the operations that must be
/// provided by a `Backend`'s command buffer.
pub trait RawCommandBuffer<B: Backend>: Clone + Send {
    /// DOC TODO
    fn begin(&mut self, flags: CommandBufferFlags);

    /// DOC TODO
    fn finish(&mut self);

    /// DOC TODO
    fn reset(&mut self, release_resources: bool);

    /// DOC TODO
    fn pipeline_barrier<'a, T>(
        &mut self,
        stages: Range<pso::PipelineStage>,
        barriers: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<Barrier<'a, B>>;

    /// DOC TODO
    fn fill_buffer(
        &mut self,
        buffer: &B::Buffer,
        range: Range<u64>,
        data: u32,
    );

    /// DOC TODO
    fn update_buffer(
        &mut self,
        buffer: &B::Buffer,
        offset: u64,
        data: &[u8],
    );

    /// Clear color image
    fn clear_color_image(
        &mut self,
        image: &B::Image,
        layout: ImageLayout,
        range: SubresourceRange,
        cv: ClearColor,
    ) {
        self.clear_color_image_raw(
            image,
            layout,
            range,
            cv.into(),
        )
    }

    /// Clear color image
    fn clear_color_image_raw(
        &mut self,
        &B::Image,
        ImageLayout,
        SubresourceRange,
        ClearColorRaw,
    );

    /// Clear depth-stencil image
    fn clear_depth_stencil_image(
        &mut self,
        image: &B::Image,
        layout: ImageLayout,
        range: SubresourceRange,
        cv: ClearDepthStencil,
    ) {
        let cv = ClearDepthStencilRaw {
            depth: cv.0,
            stencil: cv.1,
        };
        self.clear_depth_stencil_image_raw(image, layout, range, cv)
    }

    /// Clear depth-stencil image
    fn clear_depth_stencil_image_raw(
        &mut self,
        &B::Image,
        ImageLayout,
        SubresourceRange,
        ClearDepthStencilRaw,
    );

    /// DOC TODO
    fn clear_attachments<T, U>(&mut self, clears: T, rects: U)
    where
        T: IntoIterator,
        T::Item: Borrow<AttachmentClear>,
        U: IntoIterator,
        U::Item: Borrow<Rect>;

    /// DOC TODO
    fn resolve_image<T>(
        &mut self,
        src: &B::Image,
        src_layout: ImageLayout,
        dst: &B::Image,
        dst_layout: ImageLayout,
        regions: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<ImageResolve>;

    /// Bind index buffer view.
    fn bind_index_buffer(&mut self, IndexBufferView<B>);

    /// Bind vertex buffers.
    fn bind_vertex_buffers(&mut self, pso::VertexBufferSet<B>);

    /// Set the viewport parameters for the rasterizer.
    ///
    /// Every other viewport, which is not specified in this call,
    /// will be disabled.
    ///
    /// Ensure that the number of set viewports at draw time is equal
    /// (or higher) to the number specified in the bound pipeline.
    ///
    /// # Errors
    ///
    /// This function does not return an error. Invalid usage of this function
    /// will result in an error on `finish`.
    ///
    /// - Command buffer must be in recording state.
    /// - Number of viewports must be between 1 and `max_viewports`.
    /// - Only queues with graphics capability support this function.
    fn set_viewports<T>(&mut self, viewports: T)
    where
        T: IntoIterator,
        T::Item: Borrow<Viewport>;

    /// Set the scissor rectangles for the rasterizer.
    ///
    /// Every other scissor, which is not specified in this call,
    /// will be disabled.
    ///
    /// Each scissor corresponds to the viewport with the same index.
    ///
    /// Ensure that the number of set scissors at draw time is equal (or higher)
    /// to the number of viewports specified in the bound pipeline.
    ///
    /// # Errors
    ///
    /// This function does not return an error. Invalid usage of this function
    /// will result in an error on `finish`.
    ///
    /// - Command buffer must be in recording state.
    /// - Number of scissors must be between 1 and `max_viewports`.
    /// - Only queues with graphics capability support this function.
    fn set_scissors<T>(&mut self, rects: T)
    where
        T: IntoIterator,
        T::Item: Borrow<Rect>;

    /// DOC TODO
    fn set_stencil_reference(&mut self, front: StencilValue, back: StencilValue);

    /// DOC TODO
    fn set_blend_constants(&mut self, ColorValue);

    /// DOC TODO
    fn begin_renderpass<T>(
        &mut self,
        render_pass: &B::RenderPass,
        framebuffer: &B::Framebuffer,
        render_area: Rect,
        clear_values: T,
        first_subpass: SubpassContents,
    ) where
        T: IntoIterator,
        T::Item: Borrow<ClearValue>
    {
        let clear_values = clear_values
            .into_iter()
            .map(|cv| {
                match *cv.borrow() {
                    ClearValue::Color(ClearColor::Float(cv)) =>
                        ClearValueRaw { color: ClearColorRaw { float32: cv }},
                    ClearValue::Color(ClearColor::Int(cv)) =>
                        ClearValueRaw { color: ClearColorRaw { int32: cv }},
                    ClearValue::Color(ClearColor::Uint(cv)) =>
                        ClearValueRaw { color: ClearColorRaw { uint32: cv }},
                    ClearValue::DepthStencil(ClearDepthStencil(depth, stencil)) =>
                        ClearValueRaw { depth_stencil: ClearDepthStencilRaw { depth, stencil }},
                }
            });

        self.begin_renderpass_raw(
            render_pass,
            framebuffer,
            render_area,
            clear_values,
            first_subpass,
        )
    }

    /// DOC TODO
    fn begin_renderpass_raw<T>(
        &mut self,
        render_pass: &B::RenderPass,
        framebuffer: &B::Framebuffer,
        render_area: Rect,
        clear_values: T,
        first_subpass: SubpassContents,
    ) where
        T: IntoIterator,
        T::Item: Borrow<ClearValueRaw>;

    /// DOC TODO
    fn next_subpass(&mut self, contents: SubpassContents);

    /// DOC TODO
    fn end_renderpass(&mut self);

    /// Bind a graphics pipeline.
    ///
    /// # Errors
    ///
    /// This function does not return an error. Invalid usage of this function
    /// will result in an error on `finish`.
    ///
    /// - Command buffer must be in recording state.
    /// - Only queues with graphics capability support this function.
    fn bind_graphics_pipeline(&mut self, &B::GraphicsPipeline);

    /// DOC TODO
    fn bind_graphics_descriptor_sets<T>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: usize,
        sets: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<B::DescriptorSet>;

    /// Bind a compute pipeline.
    ///
    /// # Errors
    ///
    /// This function does not return an error. Invalid usage of this function
    /// will result in an error on `finish`.
    ///
    /// - Command buffer must be in recording state.
    /// - Only queues with compute capability support this function.
    fn bind_compute_pipeline(&mut self, &B::ComputePipeline);

    /// DOC TODO
    fn bind_compute_descriptor_sets<T>(
        &mut self,
        layout: &B::PipelineLayout,
        first_set: usize,
        sets: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<B::DescriptorSet>;

    /// Execute a workgroup in the compute pipeline.
    ///
    /// # Errors
    ///
    /// This function does not return an error. Invalid usage of this function
    /// will result in an error on `finish`.
    ///
    /// - Command buffer must be in recording state.
    /// - A compute pipeline must be bound using `bind_compute_pipeline`.
    /// - Only queues with compute capability support this function.
    /// - This function must be called outside of a renderpass.
    /// - `(x, y, z)` must be less than or equal to `Limits::max_compute_group_size`
    ///
    /// TODO:
    fn dispatch(&mut self, x: u32, y: u32, z: u32);

    /// DOC TODO
    fn dispatch_indirect(&mut self, buffer: &B::Buffer, offset: u64);

    /// DOC TODO
    fn copy_buffer<T>(
        &mut self,
        src: &B::Buffer,
        dst: &B::Buffer,
        regions: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<BufferCopy>;

    /// DOC TODO
    fn copy_image<T>(
        &mut self,
        src: &B::Image,
        src_layout: ImageLayout,
        dst: &B::Image,
        dst_layout: ImageLayout,
        regions: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<ImageCopy>;

    /// DOC TODO
    fn copy_buffer_to_image<T>(
        &mut self,
        src: &B::Buffer,
        dst: &B::Image,
        dst_layout: ImageLayout,
        regions: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<BufferImageCopy>;

    /// DOC TODO
    fn copy_image_to_buffer<T>(
        &mut self,
        src: &B::Image,
        src_layout: ImageLayout,
        dst: &B::Buffer,
        regions: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<BufferImageCopy>;

    /// DOC TODO
    fn draw(
        &mut self,
        vertices: Range<VertexCount>,
        instances: Range<InstanceCount>,
    );

    /// DOC TODO
    fn draw_indexed(
        &mut self,
        indices: Range<IndexCount>,
        base_vertex: VertexOffset,
        instances: Range<InstanceCount>,
    );

    /// DOC TODO
    fn draw_indirect(
        &mut self,
        buffer: &B::Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    );

    /// DOC TODO
    fn draw_indexed_indirect(
        &mut self,
        buffer: &B::Buffer,
        offset: u64,
        draw_count: u32,
        stride: u32,
    );

    /// DOC TODO
    fn begin_query(&mut self, query: Query<B>, flags: QueryControl);

    /// DOC TODO
    fn end_query(&mut self, query: Query<B>);

    /// DOC TODO
    fn reset_query_pool(&mut self, pool: &B::QueryPool, queries: Range<QueryId>);

    /// DOC TODO
    fn write_timestamp(&mut self, pso::PipelineStage, Query<B>);

    /// DOC TODO
    fn push_graphics_constants(
        &mut self,
        layout: &B::PipelineLayout,
        stages: pso::ShaderStageFlags,
        offset: u32,
        constants: &[u32],
    );

    /// DOC TODO
    fn push_compute_constants(
        &mut self,
        layout: &B::PipelineLayout,
        offset: u32,
        constants: &[u32],
    );

    /// DOC TODO
    fn execute_commands<I>(
        &mut self,
        buffers: I,
    ) where
        I: IntoIterator,
        I::Item: Borrow<B::CommandBuffer>;
}
