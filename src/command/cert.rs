pub const NAME: &str = "cert";

#[inline]
pub fn args<'a>() -> clap::Command<'a> {
    clap::Command::new(NAME)
        .about("Helper to create self-signed certificate.")
        .arg(clap::arg!(-h --help "Print help information.").display_order(0))
        .arg(
            clap::arg!(-o --output <NAME> "Output files as '<NAME>.key' and '<NAME>.crt'.")
                .display_order(1),
        )
        .arg(clap::arg!(--cn <NAME> "Set the common name.").display_order(2))
        .arg(
            clap::arg!(--dns <DNS> "Add a dns entry into alternative names.")
                .action(clap::builder::ArgAction::Append)
                .required(false)
                .display_order(3),
        )
        .arg(
            clap::arg!(--ip <IP> "Add an IP entry into alternative names.")
                .action(clap::builder::ArgAction::Append)
                .required(false)
                .display_order(4),
        )
}

#[inline]
pub fn run() -> ! {
    let matches = args().get_matches();
    run_with_matches(&matches);
}

#[inline]
pub fn run_with_matches(matches: &clap::ArgMatches) -> ! {
    let mut args = "req -x509 -newkey rsa:4096 -sha256 -days 36500 -nodes"
        .split(' ')
        .collect::<Vec<&str>>();

    let output_name = matches.get_one::<String>("output").unwrap();
    let s = format!("-keyout {output_name}.key -out {output_name}.crt");
    args.extend(s.split(' '));

    let cn = matches.get_one::<String>("cn").unwrap();
    let s = format!("-subj '/CN={cn}'");
    args.extend(s.split(' '));

    let dns_entries: Box<dyn Iterator<Item = String>> = match matches.get_many::<String>("dns") {
        Some(it) => Box::new(it.map(|s| format!("DNS:{s}"))),
        None => Box::new(std::iter::empty()),
    };
    let ip_entries: Box<dyn Iterator<Item = String>> = match matches.get_many::<String>("ip") {
        Some(it) => Box::new(it.map(|s| format!("IP:{s}"))),
        None => Box::new(std::iter::empty()),
    };
    let mut s = dns_entries
        .chain(ip_entries)
        .collect::<Vec<String>>()
        .join(",");
    if !s.is_empty() {
        s = format!("subjectAltName={s}");
        args.push("-addext");
        args.push(&s);
    }

    crate::exec("openssl", &args, &[]);
}
