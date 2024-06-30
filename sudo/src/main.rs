fn main() {
    let mut args = std::env::args().skip(1);
    let file = match args.next() {
        Some(file) => file,
        None => {
            return;
        }
    };
    let mut params = String::new();
    let mut prepend_space = false;
    for param in args {
        let param = param.replace("\"", "\\\"");
        if prepend_space {
            params.push(' ');
        }
        params.push_str(&format!("\"{}\" ", param));
        prepend_space = true;
    }

    let verb = "runas\0".encode_utf16().collect::<Vec<_>>();
    let file = file
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let params = params
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    unsafe {
        winapi::um::shellapi::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            winapi::um::winuser::SW_SHOWNORMAL,
        );
    }
}
