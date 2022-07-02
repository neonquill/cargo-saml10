use anyhow::Result;
use clap::Parser;
use probe_rs::Probe;
use probe_rs_cli_util::build_artifact;
use probe_rs_cli_util::common_options::CargoOptions;
use std::path::PathBuf;

mod elf;
mod sam;

#[derive(Parser, Debug)]
#[clap(version, about)]
struct Args {
    #[clap(short, long, value_parser,
           default_value_t = simplelog::LevelFilter::Warn)]
    log: simplelog::LevelFilter,

    #[clap(flatten)]
    cargo_options: CargoOptions,
}

fn main() -> Result<()> {
    // Remove extra arg coming from cargo, ala cargo-flash.
    let raw_args = std::env::args();
    let mut raw_args: Vec<_> = raw_args.collect();
    if raw_args.get(1) == Some(&"saml10".to_string()) {
        raw_args.remove(1);
    }

    let args = Args::parse_from(&raw_args);

    simplelog::TermLogger::init(
        args.log,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )?;

    let work_dir = PathBuf::from(".");
    let path =
        build_artifact(&work_dir, &args.cargo_options.to_cargo_options())?
            .path()
            .to_owned();

    println!("Programming {}", path.display());

    let data = elf::extract_data(&path)?;

    let saml10 = sam::Atsaml10::new();

    let probes = Probe::list_all();
    let mut probe = probes[0].open()?;

    // Attach without running any init routines.
    probe.attach_to_unspecified()?;

    print!("Erasing");
    let probe = saml10.erase(probe)?;
    println!("...Done");

    print!("Flashing");
    let probe = saml10.program(probe, &data)?;
    println!("Done");

    print!("Verifying");
    let probe = saml10.verify(probe, &data)?;
    println!("Done");

    saml10.reset(probe)?;

    Ok(())
}
