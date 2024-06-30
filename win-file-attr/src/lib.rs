use simple_logger::error;

const HIDDEN: u32 = winapi::um::winnt::FILE_ATTRIBUTE_HIDDEN;
const SYSTEM: u32 = winapi::um::winnt::FILE_ATTRIBUTE_SYSTEM;

pub unsafe fn set<S: AsRef<str>>(path: S, hidden: bool, system: bool) -> Result<(), ()> {
    let path = encode_wide_str(path);
    let mut attr = get_attribute(&path)?;
    if hidden {
        attr |= HIDDEN;
    }
    if system {
        attr |= SYSTEM;
    }
    set_attribute(&path, attr)
}

pub unsafe fn unset<S: AsRef<str>>(path: S, hidden: bool, system: bool) -> Result<(), ()> {
    let path = encode_wide_str(path);
    let mut attr = get_attribute(&path)?;
    if hidden {
        attr &= !HIDDEN;
    }
    if system {
        attr &= !SYSTEM;
    }
    set_attribute(&path, attr)
}

fn encode_wide_str<S: AsRef<str>>(s: S) -> Vec<u16> {
    s.as_ref().encode_utf16().chain(Some(0)).collect()
}

unsafe fn get_attribute(path: &[u16]) -> Result<u32, ()> {
    let result = winapi::um::fileapi::GetFileAttributesW(path.as_ptr());
    if result == winapi::um::fileapi::INVALID_FILE_ATTRIBUTES {
        let e = std::io::Error::last_os_error();
        error!("failed to get current attributes: {}", e);
        return Err(());
    }
    Ok(result)
}

unsafe fn set_attribute(path: &[u16], attr: u32) -> Result<(), ()> {
    if winapi::um::fileapi::SetFileAttributesW(path.as_ptr(), attr) == 0 {
        let e = std::io::Error::last_os_error();
        error!("failed to set attributes: {}", e);
        return Err(());
    }
    Ok(())
}
