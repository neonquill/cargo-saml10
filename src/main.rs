use clap::Parser;
use probe_rs_cli_util::common_options::CargoOptions;

#[derive(Parser, Debug)]
#[clap(version, about)]
struct Args {
    #[clap(short, long, value_parser)]
    name: String,

    #[clap(flatten)]
    cargo_options: CargoOptions,
}

fn main() {
    let args = Args::parse();

    println!("Hello, {}!", args.name);
    println!("Cargo: {:?}", args.cargo_options);
}
