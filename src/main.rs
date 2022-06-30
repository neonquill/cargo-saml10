use clap::Parser;

#[derive(Parser, Debug)]
#[clap(version, about)]
struct Args {
    #[clap(short, long, value_parser)]
    name: String,
}

fn main() {
    let args = Args::parse();

    println!("Hello, {}!", args.name);
}
