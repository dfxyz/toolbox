use std::mem::MaybeUninit;

use winapi::shared::{minwindef, winerror};
use winapi::um::{winnt, winreg};

pub const NAME: &str = "wt";

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Helper for Windows Terminal.")
        .arg(clap::arg!(-h --help "Print help information."))
        .arg(clap::arg!(-i --install "Install context menus.").conflicts_with("uninstall"))
        .arg(clap::arg!(-u --uninstall "Uninstall context menus.").conflicts_with("install"))
}

#[inline]
pub fn run() -> ! {
    let matches = args().get_matches();
    run_with_matches(&matches);
}

#[inline]
pub fn run_with_matches(matches: &clap::ArgMatches) -> ! {
    if matches.contains_id("install") {
        unsafe { install_context_menus() };
    }

    if matches.contains_id("uninstall") {
        unsafe { uninstall_context_menus() };
    }

    println!("Try '--help' for more information.");
    std::process::exit(0);
}

unsafe fn install_context_menus() -> ! {
    let command = "wt.exe -d %V";
    let admin_command =
        "powershell -WindowStyle Hidden -Command Start-Process wt.exe -ArgumentList '-d','%V' -Verb RunAs";
    let menu_text = "在此处打开 Windows Terminal(&T)";
    let admin_menu_text = "以管理员身份在此处打开 Windows Terminal(&Y)";

    for key in [
        r"Directory\shell\wt",
        r"Directory\Background\shell\wt",
        r"Drive\shell\wt",
    ] {
        match create_key(winreg::HKEY_CLASSES_ROOT, key) {
            Ok(hkey) => {
                match set_key_value(hkey, "", menu_text) {
                    Ok(_) => {}
                    Err(code) => {
                        eprintln!(
                            "error: failed to set default value for '{}' ({})",
                            key, code
                        );
                        std::process::exit(1);
                    }
                }
                match create_key(hkey, "Command") {
                    Ok(hkey) => {
                        match set_key_value(hkey, "", command) {
                            Ok(_) => {}
                            Err(code) => {
                                eprintln!(
                                    "error: failed to set default value for 'Command' of '{}' ({})",
                                    key, code
                                );
                                std::process::exit(1);
                            }
                        }
                        match close_key(hkey) {
                            Ok(_) => {}
                            Err(code) => {
                                eprintln!(
                                    "error: failed to close 'Command' of '{}' ({})",
                                    key, code
                                );
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(code) => {
                        eprintln!(
                            "error: failed to create sub-tree 'Command' for '{}' ({})",
                            key, code
                        );
                        std::process::exit(1);
                    }
                }
                match close_key(hkey) {
                    Ok(_) => {}
                    Err(code) => {
                        eprintln!("error: failed to close '{}' ({})", key, code);
                        std::process::exit(1);
                    }
                }
            }
            Err(code) => {
                eprintln!("error: failed to create tree for '{}' ({})", key, code);
                std::process::exit(1);
            }
        };
    }

    for key in [
        r"Directory\shell\wt-admin",
        r"Directory\Background\shell\wt-admin",
        r"Drive\shell\wt-admin",
    ] {
        match create_key(winreg::HKEY_CLASSES_ROOT, key) {
            Ok(hkey) => {
                match set_key_value(hkey, "", admin_menu_text) {
                    Ok(_) => {}
                    Err(code) => {
                        eprintln!(
                            "error: failed to set default value for '{}' ({})",
                            key, code
                        );
                        std::process::exit(1);
                    }
                }
                match set_key_value(hkey, "Extended", "") {
                    Ok(_) => {}
                    Err(code) => {
                        eprintln!("error: failed to set 'Extended' for '{}' ({})", key, code);
                        std::process::exit(1);
                    }
                }
                match create_key(hkey, "Command") {
                    Ok(hkey) => {
                        match set_key_value(hkey, "", admin_command) {
                            Ok(_) => {}
                            Err(code) => {
                                eprintln!(
                                    "error: failed to set default value for 'Command' of '{}' ({})",
                                    key, code
                                );
                                std::process::exit(1);
                            }
                        }
                        match close_key(hkey) {
                            Ok(_) => {}
                            Err(code) => {
                                eprintln!(
                                    "error: failed to close 'Command' of '{}' ({})",
                                    key, code
                                );
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(code) => {
                        eprintln!(
                            "error: failed to create sub-tree 'Command' for '{}' ({})",
                            key, code
                        );
                        std::process::exit(1);
                    }
                }
                match close_key(hkey) {
                    Ok(_) => {}
                    Err(code) => {
                        eprintln!("error: failed to close '{}' ({})", key, code);
                        std::process::exit(1);
                    }
                }
            }
            Err(code) => {
                eprintln!("error: failed to create tree for '{}' ({})", key, code);
                std::process::exit(1);
            }
        };
    }

    std::process::exit(0);
}

unsafe fn create_key(hkey: minwindef::HKEY, key: &str) -> Result<minwindef::HKEY, i32> {
    let key = crate::utf16_str(key);
    let mut result_hkey = MaybeUninit::uninit();
    let result = winreg::RegCreateKeyExW(
        hkey,
        key.as_ptr(),
        0,
        std::ptr::null_mut(),
        0,
        winnt::KEY_ALL_ACCESS,
        std::ptr::null_mut(),
        result_hkey.as_mut_ptr(),
        std::ptr::null_mut(),
    );
    if result == winerror::ERROR_SUCCESS as i32 {
        Ok(result_hkey.assume_init())
    } else {
        Err(result)
    }
}

unsafe fn set_key_value(hkey: minwindef::HKEY, name: &str, value: &str) -> Result<(), i32> {
    let name = crate::utf16_str(name);
    let value = crate::utf16_str(value);
    let value_byte_num = value.len() * 2;
    let result = winreg::RegSetKeyValueW(
        hkey,
        std::ptr::null_mut(),
        name.as_ptr(),
        winreg::RRF_RT_REG_SZ,
        value.as_ptr() as _,
        value_byte_num as _,
    );
    if result == winerror::ERROR_SUCCESS as i32 {
        Ok(())
    } else {
        Err(result)
    }
}

unsafe fn close_key(hkey: minwindef::HKEY) -> Result<(), i32> {
    let result = winreg::RegCloseKey(hkey);
    if result == winerror::ERROR_SUCCESS as i32 {
        Ok(())
    } else {
        Err(result)
    }
}

unsafe fn uninstall_context_menus() -> ! {
    for key in [
        r"Directory\shell\wt",
        r"Directory\shell\wt-admin",
        r"Directory\Background\shell\wt",
        r"Directory\Background\shell\wt-admin",
        r"Drive\shell\wt",
        r"Drive\shell\wt-admin",
    ] {
        match delete_tree(key) {
            Ok(_) => {}
            Err(code) => {
                eprintln!("error: failed to delete '{}' ({})", key, code);
                std::process::exit(1);
            }
        }
    }
    std::process::exit(0);
}

unsafe fn delete_tree(key: &str) -> Result<(), i32> {
    let key = crate::utf16_str(key);
    let result = winreg::RegDeleteTreeW(winreg::HKEY_CLASSES_ROOT, key.as_ptr());
    if result == winerror::ERROR_SUCCESS as i32 || result == winerror::ERROR_FILE_NOT_FOUND as i32 {
        Ok(())
    } else {
        Err(result)
    }
}
