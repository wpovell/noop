mod args;
mod err;
mod intercept;

fn main() {
    match args::parse() {
        Err(e) => {
            eprintln!("Malformed arguments: {}\n", e);
            args::usage(1);
        }
        Ok(args) => {
            if let Err(e) = intercept::start(&args) {
                eprintln!("Error: {}", e);
            }
        }
    }
}
