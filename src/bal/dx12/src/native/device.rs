//! Device

use super::com::WeakPtr;

use std::ops::Range;
use std::ptr;
use winapi::um::d3d12;
use winapi::Interface;
use {D3DResult, FeatureLevel, NodeMask, TextureAddressMode};

use super::command_list::CmdListType;
use super::descriptor::{CpuDescriptor, HeapFlags, HeapType};
use super::{pso, query, queue};
use super::{
    Blob, CachedPSO, CommandAllocator, CommandQueue, DescriptorHeap, GraphicsCommandList,
    PipelineState, QueryHeap, RootSignature, Shader,
};

pub type Device = WeakPtr<d3d12::ID3D12Device>;

impl Device {
    pub fn create<I: Interface>(
        adapter: WeakPtr<I>,
        feature_level: FeatureLevel,
    ) -> D3DResult<Self> {
        let device = Device::null();
        let hr = unsafe {
            d3d12::D3D12CreateDevice(
                adapter.as_unknown() as *const _ as *mut _,
                feature_level as _,
                &d3d12::ID3D12Device::uuidof(),
                &mut device.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };

        (device, hr)
    }

    pub fn create_command_allocator(&self, list_type: CmdListType) -> D3DResult<CommandAllocator> {
        let allocator = CommandAllocator::null();
        let hr = unsafe {
            self.CreateCommandAllocator(
                list_type as _,
                &d3d12::ID3D12CommandAllocator::uuidof(),
                &mut allocator.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };

        (allocator, hr)
    }

    pub fn create_command_queue(
        &self,
        list_type: CmdListType,
        priority: queue::Priority,
        flags: queue::CommandQueueFlags,
        node_mask: NodeMask,
    ) -> D3DResult<CommandQueue> {
        let desc = d3d12::D3D12_COMMAND_QUEUE_DESC {
            Type: list_type as _,
            Priority: priority as _,
            Flags: flags.bits(),
            NodeMask: node_mask,
        };

        let queue = CommandQueue::null();
        let hr = unsafe {
            self.CreateCommandQueue(
                &desc,
                &d3d12::ID3D12CommandQueue::uuidof(),
                &mut queue.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };

        (queue, hr)
    }

    pub fn create_descriptor_heap(
        &self,
        num_descriptors: u32,
        heap_type: HeapType,
        flags: HeapFlags,
        node_mask: NodeMask,
    ) -> D3DResult<DescriptorHeap> {
        let desc = d3d12::D3D12_DESCRIPTOR_HEAP_DESC {
            Type: heap_type as _,
            NumDescriptors: num_descriptors,
            Flags: flags.bits(),
            NodeMask: node_mask,
        };

        let heap = DescriptorHeap::null();
        let hr = unsafe {
            self.CreateDescriptorHeap(
                &desc,
                &d3d12::ID3D12DescriptorHeap::uuidof(),
                &mut heap.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };

        (heap, hr)
    }

    pub fn get_descriptor_increment_size(&self, heap_type: HeapType) -> u32 {
        unsafe { self.GetDescriptorHandleIncrementSize(heap_type as _) }
    }

    pub fn create_graphics_command_list(
        &self,
        list_type: CmdListType,
        allocator: CommandAllocator,
        initial: PipelineState,
        node_mask: NodeMask,
    ) -> D3DResult<GraphicsCommandList> {
        let command_list = GraphicsCommandList::null();
        let hr = unsafe {
            self.CreateCommandList(
                node_mask,
                list_type as _,
                allocator.as_mut_ptr(),
                initial.as_mut_ptr(),
                &d3d12::ID3D12GraphicsCommandList::uuidof(),
                &mut command_list.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };

        (command_list, hr)
    }

    pub fn create_query_heap(
        &self,
        heap_ty: query::HeapType,
        count: u32,
        node_mask: NodeMask,
    ) -> D3DResult<QueryHeap> {
        let desc = d3d12::D3D12_QUERY_HEAP_DESC {
            Type: heap_ty as _,
            Count: count,
            NodeMask: node_mask,
        };

        let query_heap = QueryHeap::null();
        let hr = unsafe {
            self.CreateQueryHeap(
                &desc,
                &d3d12::ID3D12QueryHeap::uuidof(),
                &mut query_heap.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };

        (query_heap, hr)
    }

    pub fn create_graphics_pipeline_state(
        &self,
        root_signature: RootSignature,
        vs: Shader,
        ps: Shader,
        gs: Shader,
        hs: Shader,
        ds: Shader,
        node_mask: NodeMask,
        cached_pso: CachedPSO,
        flags: pso::PipelineStateFlags,
    ) -> D3DResult<PipelineState> {
        unimplemented!()
    }

    pub fn create_compute_pipeline_state(
        &self,
        root_signature: RootSignature,
        cs: Shader,
        node_mask: NodeMask,
        cached_pso: CachedPSO,
        flags: pso::PipelineStateFlags,
    ) -> D3DResult<PipelineState> {
        let pipeline = PipelineState::null();
        let desc = d3d12::D3D12_COMPUTE_PIPELINE_STATE_DESC {
            pRootSignature: root_signature.as_mut_ptr(),
            CS: *cs,
            NodeMask: node_mask,
            CachedPSO: *cached_pso,
            Flags: flags.bits(),
        };

        let hr = unsafe {
            self.CreateComputePipelineState(
                &desc,
                &d3d12::ID3D12PipelineState::uuidof(),
                &mut pipeline.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };

        (pipeline, hr)
    }

    pub fn create_sampler(
        &self,
        sampler: CpuDescriptor,
        filter: d3d12::D3D12_FILTER,
        address_mode: TextureAddressMode,
        mip_lod_bias: f32,
        max_anisotropy: u32,
        comparison_op: d3d12::D3D12_COMPARISON_FUNC,
        border_color: [f32; 4],
        lod: Range<f32>,
    ) {
        let desc = d3d12::D3D12_SAMPLER_DESC {
            Filter: filter,
            AddressU: address_mode[0],
            AddressV: address_mode[1],
            AddressW: address_mode[2],
            MipLODBias: mip_lod_bias,
            MaxAnisotropy: max_anisotropy,
            ComparisonFunc: comparison_op,
            BorderColor: border_color,
            MinLOD: lod.start,
            MaxLOD: lod.end,
        };

        unsafe {
            self.CreateSampler(&desc, sampler);
        }
    }

    pub fn create_root_signature(
        &self,
        blob: Blob,
        node_mask: NodeMask,
    ) -> D3DResult<RootSignature> {
        let signature = RootSignature::null();
        let hr = unsafe {
            self.CreateRootSignature(
                node_mask,
                blob.GetBufferPointer(),
                blob.GetBufferSize(),
                &d3d12::ID3D12RootSignature::uuidof(),
                &mut signature.as_mut_ptr() as *mut *mut _ as *mut *mut _,
            )
        };

        (signature, hr)
    }
}