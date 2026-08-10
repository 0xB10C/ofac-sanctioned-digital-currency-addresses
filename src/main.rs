use std::env;

fn main() {
    if let Err(err) = ofac_sanctioned_digital_currency_addresses::parse_args(env::args_os().skip(1))
        .and_then(ofac_sanctioned_digital_currency_addresses::run)
    {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
