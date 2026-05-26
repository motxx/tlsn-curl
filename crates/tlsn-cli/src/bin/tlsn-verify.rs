fn main() {
    let result = tlsn_curl_cli::run_verify_cli(std::env::args().skip(1));
    if !result.stdout.is_empty() {
        print!("{}", result.stdout);
    }
    if !result.stderr.is_empty() {
        eprint!("{}", result.stderr);
    }
    std::process::exit(result.code);
}
