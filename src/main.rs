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
    #[clap(flatten)]
    cargo_options: CargoOptions,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let work_dir = PathBuf::from(".");
    let path =
        build_artifact(&work_dir, &args.cargo_options.to_cargo_options())?
            .path()
            .to_owned();

    let data = elf::extract_data(&path)?;

    let saml10 = sam::Atsaml10::new();

    let probes = Probe::list_all();
    let mut probe = probes[0].open()?;

    // Attach without running any init routines.
    probe.attach_to_unspecified()?;

    let probe = saml10.erase(probe)?;

    saml10.program(probe, &data)?;

    // XXX Need to reset.

    Ok(())
}
