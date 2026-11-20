#[link(name="kernel32")]
extern "system" {
    pub fn CreateMutexW(attributes:*const std::ffi::c_void, initial:i32, name:*const u16)->isize;
    pub fn GetLastError()->u32;
    pub fn CloseHandle(handle:isize)->i32;
}
