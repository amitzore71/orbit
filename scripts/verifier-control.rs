//! Deliberate invalid free in a separate disposable process, to calibrate AppVerifier.
//! Never linked into the application. Run only with Heaps verification enabled.
use std::ffi::c_void;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetProcessHeap() -> *mut c_void;
    fn HeapAlloc(heap: *mut c_void, flags: u32, bytes: usize) -> *mut c_void;
    fn HeapFree(heap: *mut c_void, flags: u32, memory: *mut c_void) -> i32;
    fn SetErrorMode(mode: u32) -> u32;
}
fn main() {
    unsafe {
        SetErrorMode(0x8003); // suppress Windows error UI for this expected failure
        let heap = GetProcessHeap();
        let allocation = HeapAlloc(heap, 0, 64);
        assert!(!allocation.is_null());
        assert_ne!(HeapFree(heap, 0, allocation), 0);
        HeapFree(heap, 0, allocation); // intentional double free: positive test control
    }
}
