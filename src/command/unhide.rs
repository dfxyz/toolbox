use winapi::um::fileapi::{GetFileAttributesW, SetFileAttributesW, INVALID_FILE_ATTRIBUTES};
use winapi::um::winnt::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM};

pub const NAME: &str = "unhide";

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Unhide the given file(s).")
        .trailing_var_arg(true)
        .arg(clap::arg!(-h --help "Print help information."))
        .arg(clap::arg!(-s --system "Also unset the \"system\" attribute."))
        .arg(clap::arg!(<FILE>... "File(s) to unhide."))
}

#[inline]
pub fn run() -> ! {
    let matches = args().get_matches();
    run_with_matches(&matches);
}

#[inline]
pub fn run_with_matches(matches: &clap::ArgMatches) -> ! {
    let unset_system_attr = matches.is_present("system");
    matches.values_of("FILE").unwrap().for_each(|f| {
        let file = crate::utf16_str(f);
        let file = file.as_ptr();
        unsafe {
            let mut attr = GetFileAttributesW(file);
            if attr == INVALID_FILE_ATTRIBUTES {
                eprintln!("ERROR: cannot get the attributes of \"{}\".", f);
                return;
            }
            attr &= !FILE_ATTRIBUTE_HIDDEN;
            if unset_system_attr {
                attr &= !FILE_ATTRIBUTE_SYSTEM;
            }
            if SetFileAttributesW(file, attr) == 0 {
                eprintln!("ERROR: failed to unhide \"{}\".", f);
            }
        }
    });
    std::process::exit(0);
}
