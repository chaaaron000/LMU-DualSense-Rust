use std::{
    ffi::OsStr,
    mem::{size_of, size_of_val},
    os::windows::ffi::OsStrExt,
    ptr::{copy_nonoverlapping, null},
    sync::atomic::{AtomicI32, Ordering},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, INVALID_HANDLE_VALUE,
        WAIT_OBJECT_0, WAIT_TIMEOUT,
    },
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        Memory::{
            CreateFileMappingW, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile,
            FILE_MAP_ALL_ACCESS, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
        },
        Threading::{
            CreateEventW, OpenEventW, OpenProcess, SetEvent, WaitForSingleObject,
            PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE, SYNCHRONIZATION_SYNCHRONIZE,
        },
    },
};

use super::lmu_layout::SNAPSHOT_SIZE;

const LMU_PROCESS_NAME: &str = "Le Mans Ultimate.exe";
const LMU_UPDATE_EVENT: &str = "LMU_Data_Event";
const LOCK_MAPPING_NAME: &str = "LMU_SharedMemoryLockData";
const LOCK_EVENT_NAME: &str = "LMU_SharedMemoryLockEvent";
const LOCK_SPINS: usize = 4_000;
const LOCK_TIMEOUT: Duration = Duration::from_millis(25);

pub struct LmuSharedMemory {
    process: OwnedHandle,
    _update_event: OwnedHandle,
    _data_mapping: OwnedHandle,
    data_view: MappedView,
    lock: SharedMemoryLock,
}

impl LmuSharedMemory {
    pub fn open(mapping_name: &str) -> Result<Self> {
        let process_id = find_process_id(LMU_PROCESS_NAME)?
            .with_context(|| format!("{LMU_PROCESS_NAME} is not running"))?;
        let process = unsafe {
            OwnedHandle::new(OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                process_id,
            ))
        }
        .context("failed to open the LMU process")?;

        let update_event_name = wide_string(LMU_UPDATE_EVENT);
        let update_event = unsafe {
            OwnedHandle::new(OpenEventW(
                SYNCHRONIZATION_SYNCHRONIZE,
                0,
                update_event_name.as_ptr(),
            ))
        }
        .context("failed to open LMU_Data_Event")?;

        let mapping_name = wide_string(mapping_name);
        let data_mapping =
            unsafe { OwnedHandle::new(OpenFileMappingW(FILE_MAP_READ, 0, mapping_name.as_ptr())) }
                .context("failed to open LMU_Data")?;
        let data_view = unsafe { MappedView::map(&data_mapping, FILE_MAP_READ, SNAPSHOT_SIZE) }
            .context("failed to map LMU_Data")?;
        let lock = SharedMemoryLock::open().context("failed to initialize the LMU shared lock")?;

        Ok(Self {
            process,
            _update_event: update_event,
            _data_mapping: data_mapping,
            data_view,
            lock,
        })
    }

    pub fn is_process_alive(&self) -> Result<bool> {
        match unsafe { WaitForSingleObject(self.process.raw(), 0) } {
            WAIT_TIMEOUT => Ok(true),
            WAIT_OBJECT_0 => Ok(false),
            result => bail!("LMU process wait failed with status {result}"),
        }
    }

    pub fn copy_snapshot(&self, target: &mut [u8]) -> Result<()> {
        if target.len() != SNAPSHOT_SIZE {
            bail!(
                "snapshot target has size {}, expected {SNAPSHOT_SIZE}",
                target.len()
            );
        }

        let _guard = self.lock.acquire(LOCK_TIMEOUT)?;
        unsafe {
            copy_nonoverlapping(
                self.data_view.as_ptr().cast::<u8>(),
                target.as_mut_ptr(),
                SNAPSHOT_SIZE,
            );
        }
        Ok(())
    }
}

struct SharedMemoryLock {
    _mapping: OwnedHandle,
    view: MappedView,
    event: OwnedHandle,
}

impl SharedMemoryLock {
    fn open() -> Result<Self> {
        let mapping_name = wide_string(LOCK_MAPPING_NAME);
        let (mapping, create_error) = unsafe {
            let handle = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                null(),
                PAGE_READWRITE,
                0,
                size_of::<LockData>() as u32,
                mapping_name.as_ptr(),
            );
            (OwnedHandle::new(handle), GetLastError())
        };
        let mapping = mapping.context("failed to create/open LMU lock mapping")?;
        let already_existed = create_error == ERROR_ALREADY_EXISTS;
        let view = unsafe { MappedView::map(&mapping, FILE_MAP_ALL_ACCESS, size_of::<LockData>()) }
            .context("failed to map LMU lock data")?;

        let event_name = wide_string(LOCK_EVENT_NAME);
        let event = unsafe { OwnedHandle::new(CreateEventW(null(), 0, 0, event_name.as_ptr())) }
            .context("failed to create/open LMU lock event")?;

        let lock = Self {
            _mapping: mapping,
            view,
            event,
        };
        if !already_existed {
            lock.data().waiters.store(0, Ordering::Release);
            lock.data().busy.store(0, Ordering::Release);
        }
        Ok(lock)
    }

    fn acquire(&self, timeout: Duration) -> Result<SharedMemoryGuard<'_>> {
        for _ in 0..LOCK_SPINS {
            if self
                .data()
                .busy
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return Ok(SharedMemoryGuard { lock: self });
            }
            std::hint::spin_loop();
        }

        self.data().waiters.fetch_add(1, Ordering::AcqRel);
        let deadline = Instant::now() + timeout;
        loop {
            if self
                .data()
                .busy
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                self.data().waiters.fetch_sub(1, Ordering::AcqRel);
                return Ok(SharedMemoryGuard { lock: self });
            }

            let now = Instant::now();
            if now >= deadline {
                self.data().waiters.fetch_sub(1, Ordering::AcqRel);
                bail!("timed out waiting for the LMU shared-memory lock");
            }
            let remaining_ms = (deadline - now).as_millis().clamp(1, u128::from(u32::MAX)) as u32;
            match unsafe { WaitForSingleObject(self.event.raw(), remaining_ms) } {
                WAIT_OBJECT_0 => {}
                WAIT_TIMEOUT => {
                    self.data().waiters.fetch_sub(1, Ordering::AcqRel);
                    bail!("timed out waiting for the LMU shared-memory lock");
                }
                result => {
                    self.data().waiters.fetch_sub(1, Ordering::AcqRel);
                    bail!("LMU lock wait failed with status {result}");
                }
            }
        }
    }

    fn data(&self) -> &LockData {
        unsafe { &*self.view.as_ptr().cast::<LockData>() }
    }
}

struct SharedMemoryGuard<'a> {
    lock: &'a SharedMemoryLock,
}

impl Drop for SharedMemoryGuard<'_> {
    fn drop(&mut self) {
        self.lock.data().busy.store(0, Ordering::Release);
        if self.lock.data().waiters.load(Ordering::Acquire) > 0 {
            unsafe {
                SetEvent(self.lock.event.raw());
            }
        }
    }
}

#[repr(C)]
struct LockData {
    waiters: AtomicI32,
    busy: AtomicI32,
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    unsafe fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(handle))
        }
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

struct MappedView(MEMORY_MAPPED_VIEW_ADDRESS);

impl MappedView {
    unsafe fn map(mapping: &OwnedHandle, access: u32, size: usize) -> Option<Self> {
        let view = MapViewOfFile(mapping.raw(), access, 0, 0, size);
        if view.Value.is_null() {
            None
        } else {
            Some(Self(view))
        }
    }

    fn as_ptr(&self) -> *mut std::ffi::c_void {
        self.0.Value
    }
}

impl Drop for MappedView {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.0);
        }
    }
}

fn find_process_id(executable_name: &str) -> Result<Option<u32>> {
    let snapshot = unsafe { OwnedHandle::new(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)) }
        .context("failed to enumerate Windows processes")?;
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..PROCESSENTRY32W::default()
    };

    if unsafe { Process32FirstW(snapshot.raw(), &mut entry) } == 0 {
        return Ok(None);
    }

    loop {
        let length = entry
            .szExeFile
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(entry.szExeFile.len());
        let name = String::from_utf16_lossy(&entry.szExeFile[..length]);
        if name.eq_ignore_ascii_case(executable_name) {
            return Ok(Some(entry.th32ProcessID));
        }

        if unsafe { Process32NextW(snapshot.raw(), &mut entry) } == 0 {
            break;
        }
    }
    Ok(None)
}

fn wide_string(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

const _: () = assert!(size_of::<LockData>() == 8);
const _: () = assert!(size_of_val(&[0_u8; SNAPSHOT_SIZE]) == SNAPSHOT_SIZE);
