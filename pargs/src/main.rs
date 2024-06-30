use simple_logger::{custom, warn};

fn main() {
    let args = std::env::args();
    if args.len() == 0 {
        warn!("no arguments provided");
        return;
    }

    for (i, arg) in args.enumerate() {
        let title = format!("Param #{}", i);
        custom!(title=title; "{}", arg);
    }
}
