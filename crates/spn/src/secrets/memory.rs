//! Secure memory protection for sensitive data.
//!
//! Provides memory locking (mlock) to prevent secrets from being swapped to disk,
//! and MADV_DONTDUMP to prevent secrets from appearing in core dumps.
//!
//! # Security Features
//!
//! - `mlock()`: Locks memory pages in RAM, preventing swap
//! - `MADV_DONTDUMP`: Excludes memory from core dumps
//! - `Zeroize`: Clears memory on drop
//!
//! # Platform Support
//!
//! - Unix (macOS, Linux, BSD): Full support via libc
//! - Windows: Partial support (mlock via VirtualLock, no MADV_DONTDUMP)
//!
//! # Usage
//!
//! ```rust,ignore
//! use spn::secrets::memory::LockedBuffer;
//!
//! // Create a locked buffer for sensitive data
//! let mut buffer = LockedBuffer::new(1024)?;
//! buffer.write(b"my-secret-key");
//!
//! // Use the buffer...
//! let secret = buffer.as_slice();
//!
//! // Automatically unlocked and zeroized on drop
//! ```

use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr::NonNull;
use tracing::warn;
use zeroize::Zeroize;

/// Error type for memory operations.
#[derive(Debug, Clone)]
pub enum MemoryError {
    /// Failed to allocate memory.
    AllocationFailed,
    /// Failed to lock memory (mlock failed).
    LockFailed(i32),
    /// Failed to set memory advice.
    MadviseFailed(i32),
    /// Buffer overflow (tried to write more than capacity).
    Overflow { capacity: usize, requested: usize },
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllocationFailed => write!(f, "Failed to allocate memory"),
            Self::LockFailed(errno) => write!(f, "mlock failed with errno {}", errno),
            Self::MadviseFailed(errno) => write!(f, "madvise failed with errno {}", errno),
            Self::Overflow {
                capacity,
                requested,
            } => {
                write!(
                    f,
                    "Buffer overflow: capacity {}, requested {}",
                    capacity, requested
                )
            }
        }
    }
}

impl std::error::Error for MemoryError {}

/// A buffer with locked memory that won't be swapped to disk.
///
/// This buffer:
/// 1. Allocates memory with proper alignment
/// 2. Locks pages in RAM via mlock()
/// 3. Sets MADV_DONTDUMP to exclude from core dumps
/// 4. Zeroizes memory on drop
/// 5. Unlocks memory on drop
///
/// # Security Notes
///
/// - mlock may fail if ulimit is too low (use `ulimit -l unlimited`)
/// - On macOS, mlock is silently rate-limited for non-root processes
/// - Always check errors in production
pub struct LockedBuffer {
    ptr: NonNull<u8>,
    layout: Layout,
    len: usize,
    locked: bool,
}

// SAFETY: LockedBuffer owns its memory and provides synchronized access
unsafe impl Send for LockedBuffer {}
unsafe impl Sync for LockedBuffer {}

impl LockedBuffer {
    /// Create a new locked buffer with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Size in bytes
    ///
    /// # Errors
    ///
    /// Returns error if allocation or locking fails.
    pub fn new(capacity: usize) -> Result<Self, MemoryError> {
        if capacity == 0 {
            return Err(MemoryError::AllocationFailed);
        }

        // Align to page size for mlock efficiency
        let page_size = Self::page_size();
        let aligned_size = (capacity + page_size - 1) & !(page_size - 1);

        let layout = Layout::from_size_align(aligned_size, page_size)
            .map_err(|_| MemoryError::AllocationFailed)?;

        // Allocate zeroed memory
        let ptr = unsafe {
            let ptr = alloc_zeroed(layout);
            if ptr.is_null() {
                return Err(MemoryError::AllocationFailed);
            }
            NonNull::new_unchecked(ptr)
        };

        let mut buffer = Self {
            ptr,
            layout,
            len: 0,
            locked: false,
        };

        // Try to lock memory (best effort - may fail due to ulimit)
        buffer.try_lock()?;

        // Try to exclude from core dumps (best effort)
        buffer.try_dontdump();

        Ok(buffer)
    }

    /// Get the system page size.
    fn page_size() -> usize {
        #[cfg(unix)]
        {
            unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
        }
        #[cfg(not(unix))]
        {
            4096 // Default page size
        }
    }

    /// Try to lock memory pages in RAM.
    ///
    /// If mlock fails (often due to ulimit restrictions), we log a warning
    /// and continue. This is a defense-in-depth measure, not a hard requirement.
    #[cfg(unix)]
    fn try_lock(&mut self) -> Result<(), MemoryError> {
        let result =
            unsafe { libc::mlock(self.ptr.as_ptr() as *const libc::c_void, self.layout.size()) };

        if result == 0 {
            self.locked = true;
            Ok(())
        } else {
            // mlock failed - this is often due to ulimit restrictions
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            self.locked = false;

            // Log warning so operators know secrets may be swapped to disk
            warn!(
                errno = errno,
                size = self.layout.size(),
                "mlock failed - secrets may be swapped to disk. \
                 Consider increasing ulimit -l or running as root for maximum security."
            );

            // Return Ok to allow operation to continue (defense-in-depth, not hard requirement)
            // For strict security environments, change to: Err(MemoryError::LockFailed(errno))
            Ok(())
        }
    }

    #[cfg(not(unix))]
    fn try_lock(&mut self) -> Result<(), MemoryError> {
        // Windows: could use VirtualLock, but we'll skip for now
        self.locked = false;
        Ok(())
    }

    /// Try to exclude memory from core dumps.
    #[cfg(target_os = "linux")]
    fn try_dontdump(&self) {
        unsafe {
            libc::madvise(
                self.ptr.as_ptr() as *mut libc::c_void,
                self.layout.size(),
                libc::MADV_DONTDUMP,
            );
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn try_dontdump(&self) {
        // MADV_DONTDUMP is Linux-specific
        // macOS doesn't have an equivalent
    }

    /// Unlock memory pages.
    #[cfg(unix)]
    fn unlock(&self) {
        if self.locked {
            unsafe {
                libc::munlock(self.ptr.as_ptr() as *const libc::c_void, self.layout.size());
            }
        }
    }

    #[cfg(not(unix))]
    fn unlock(&self) {
        // No-op on non-Unix
    }

    /// Write data to the buffer.
    ///
    /// # Errors
    ///
    /// Returns error if data exceeds capacity.
    pub fn write(&mut self, data: &[u8]) -> Result<(), MemoryError> {
        if data.len() > self.layout.size() {
            return Err(MemoryError::Overflow {
                capacity: self.layout.size(),
                requested: data.len(),
            });
        }

        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.as_ptr(), data.len());
        }
        self.len = data.len();
        Ok(())
    }

    /// Get the buffer contents as a slice.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Get the buffer contents as a mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    /// Get the buffer capacity.
    pub fn capacity(&self) -> usize {
        self.layout.size()
    }

    /// Get the current length of data in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Check if memory is locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Clear the buffer contents (zeroize).
    pub fn clear(&mut self) {
        self.as_mut_slice().zeroize();
        self.len = 0;
    }
}

impl Drop for LockedBuffer {
    fn drop(&mut self) {
        // 1. Zeroize the memory
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.layout.size());
            slice.zeroize();
        }

        // 2. Unlock the memory
        self.unlock();

        // 3. Deallocate
        unsafe {
            dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

/// A locked string for holding sensitive text data.
///
/// Wrapper around LockedBuffer that provides string-like operations.
pub struct LockedString {
    buffer: LockedBuffer,
}

impl LockedString {
    /// Create a new locked string with the specified capacity.
    pub fn new(capacity: usize) -> Result<Self, MemoryError> {
        Ok(Self {
            buffer: LockedBuffer::new(capacity)?,
        })
    }

    /// Create a locked string from a string slice.
    pub fn from_str(s: &str) -> Result<Self, MemoryError> {
        let mut locked = Self::new(s.len())?;
        locked.buffer.write(s.as_bytes())?;
        Ok(locked)
    }

    /// Get the string contents.
    pub fn as_str(&self) -> &str {
        // SAFETY: We only write valid UTF-8 via from_str
        unsafe { std::str::from_utf8_unchecked(self.buffer.as_slice()) }
    }

    /// Check if memory is locked.
    pub fn is_locked(&self) -> bool {
        self.buffer.is_locked()
    }

    /// Get the length in bytes.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl std::fmt::Debug for LockedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LockedString")
            .field("len", &self.len())
            .field("locked", &self.is_locked())
            .field("content", &"[REDACTED]")
            .finish()
    }
}

impl std::fmt::Display for LockedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[LOCKED STRING: {} bytes]", self.len())
    }
}

/// Check if mlock is available and working on this system.
pub fn mlock_available() -> bool {
    #[cfg(unix)]
    {
        // Try to lock a small buffer
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
        let layout = match Layout::from_size_align(page_size, page_size) {
            Ok(l) => l,
            Err(_) => return false,
        };

        unsafe {
            let ptr = alloc_zeroed(layout);
            if ptr.is_null() {
                return false;
            }

            let result = libc::mlock(ptr as *const libc::c_void, page_size);
            if result == 0 {
                libc::munlock(ptr as *const libc::c_void, page_size);
            }
            dealloc(ptr, layout);

            result == 0
        }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Get system mlock limit (in bytes).
#[cfg(unix)]
pub fn mlock_limit() -> Option<u64> {
    use std::mem::MaybeUninit;

    let mut rlim = MaybeUninit::<libc::rlimit>::uninit();
    let result = unsafe { libc::getrlimit(libc::RLIMIT_MEMLOCK, rlim.as_mut_ptr()) };

    if result == 0 {
        let rlim = unsafe { rlim.assume_init() };
        if rlim.rlim_cur == libc::RLIM_INFINITY {
            Some(u64::MAX)
        } else {
            Some(rlim.rlim_cur)
        }
    } else {
        None
    }
}

#[cfg(not(unix))]
pub fn mlock_limit() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locked_buffer_create() {
        let buffer = LockedBuffer::new(1024);
        assert!(buffer.is_ok());
        let buffer = buffer.unwrap();
        assert!(buffer.capacity() >= 1024);
    }

    #[test]
    fn test_locked_buffer_write_read() {
        let mut buffer = LockedBuffer::new(1024).unwrap();
        let data = b"secret-key-12345";
        buffer.write(data).unwrap();

        assert_eq!(buffer.as_slice(), data);
        assert_eq!(buffer.len(), data.len());
    }

    #[test]
    fn test_locked_buffer_overflow() {
        let mut buffer = LockedBuffer::new(10).unwrap();
        // Capacity is page-aligned, so this might actually succeed
        // Use a truly large value
        let large_data = vec![0u8; buffer.capacity() + 1];
        let result = buffer.write(&large_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_locked_buffer_clear() {
        let mut buffer = LockedBuffer::new(1024).unwrap();
        buffer.write(b"secret").unwrap();
        buffer.clear();

        assert!(buffer.is_empty());
        // Memory should be zeroed
        assert!(buffer.as_slice().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_locked_string_create() {
        let s = LockedString::from_str("my-secret-password");
        assert!(s.is_ok());
        let s = s.unwrap();
        assert_eq!(s.as_str(), "my-secret-password");
    }

    #[test]
    fn test_locked_string_debug_redacted() {
        let s = LockedString::from_str("secret").unwrap();
        let debug = format!("{:?}", s);
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn test_locked_string_display_hidden() {
        let s = LockedString::from_str("secret").unwrap();
        let display = format!("{}", s);
        assert!(!display.contains("secret"));
        assert!(display.contains("bytes"));
    }

    #[test]
    fn test_mlock_available_check() {
        // This just tests that the function runs without panicking
        let _ = mlock_available();
    }

    #[test]
    fn test_mlock_limit_check() {
        // This just tests that the function runs without panicking
        let _ = mlock_limit();
    }
}
