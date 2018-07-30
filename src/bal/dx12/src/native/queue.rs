use super::com::WeakPtr;
use winapi::um::d3d12;

#[repr(u32)]
pub enum Priority {
    Normal = d3d12::D3D12_COMMAND_QUEUE_PRIORITY_NORMAL,
    High = d3d12::D3D12_COMMAND_QUEUE_PRIORITY_HIGH,
    GlobalRealtime = d3d12::D3D12_COMMAND_QUEUE_PRIORITY_GLOBAL_REALTIME,
}

bitflags! {
    pub struct CommandQueueFlags: u32 {
        const DISABLE_GPU_TIMEOUT = d3d12::D3D12_COMMAND_QUEUE_FLAG_DISABLE_GPU_TIMEOUT;
    }
}

pub type CommandQueue = WeakPtr<d3d12::ID3D12CommandQueue>;

impl CommandQueue {}