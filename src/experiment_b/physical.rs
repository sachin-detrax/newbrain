#[cfg(windows)]
mod windows_platform {
    use std::ffi::c_void;
    use std::mem::size_of;

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }

    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    pub(super) fn current_working_set_bytes() -> Option<u64> {
        let mut counters = ProcessMemoryCounters {
            cb: u32::try_from(size_of::<ProcessMemoryCounters>()).ok()?,
            page_fault_count: 0,
            peak_working_set_size: 0,
            working_set_size: 0,
            quota_peak_paged_pool_usage: 0,
            quota_paged_pool_usage: 0,
            quota_peak_non_paged_pool_usage: 0,
            quota_non_paged_pool_usage: 0,
            pagefile_usage: 0,
            peak_pagefile_usage: 0,
        };
        let size = counters.cb;
        let success = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) };
        (success != 0).then_some(counters.working_set_size as u64)
    }
}

#[cfg(target_os = "linux")]
fn platform_resident_memory_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let kib: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return kib.checked_mul(1024);
        }
    }
    None
}

#[cfg(windows)]
fn platform_resident_memory_bytes() -> Option<u64> {
    windows_platform::current_working_set_bytes()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn platform_resident_memory_bytes() -> Option<u64> {
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryState {
    Measured,
    Missing,
}

impl MemoryState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Measured => "measured",
            Self::Missing => "missing",
        }
    }
}

pub fn sample_resident_memory_bytes() -> (Option<u64>, MemoryState) {
    match platform_resident_memory_bytes() {
        Some(bytes) => (Some(bytes), MemoryState::Measured),
        None => (None, MemoryState::Missing),
    }
}
