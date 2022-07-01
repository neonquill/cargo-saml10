use anyhow::{anyhow, Result};
use clap::Parser;
use object::elf::{FileHeader32, PT_LOAD};
use object::read::elf::ProgramHeader;
use object::{Endianness, Object, ObjectSection};
use probe_rs::config::MemoryRange;
use probe_rs::Probe;
use probe_rs_cli_util::build_artifact;
use probe_rs_cli_util::common_options::CargoOptions;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[clap(version, about)]
struct Args {
    #[clap(flatten)]
    cargo_options: CargoOptions,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct DataChunk {
    address: u32,
    segment_offset: u64,
    segment_filesize: u64,
}

fn extract_data(path: &Path) -> Result<Vec<DataChunk>> {
    // Pull the raw bytes from the elf file.
    let bin_data = std::fs::read(path)?;
    let obj_file =
        object::read::elf::ElfFile::<FileHeader32<Endianness>>::parse(
            &*bin_data,
        )?;

    let endian = obj_file.endian();
    let mut extracted_data = Vec::new();

    for segment in obj_file.raw_segments() {
        let p_type = segment.p_type(endian);
        let p_paddr = segment.p_paddr(endian);
        let p_vaddr = segment.p_vaddr(endian);

        let segment_data = segment
            .data(endian, &*bin_data)
            .map_err(|_| anyhow!("Failed to access data in segment"))?;

        if segment_data.is_empty() || p_type != PT_LOAD {
            continue;
        }
        println!(
            "Loadable Segment physical {:x}, virtual {:x}",
            p_paddr, p_vaddr
        );

        let (segment_offset, segment_filesize) = segment.file_range(endian);

        let sector: core::ops::Range<u64> =
            segment_offset..segment_offset + segment_filesize;

        let mut found = false;
        for section in obj_file.sections() {
            let (section_offset, section_filesize) = match section.file_range()
            {
                Some(range) => range,
                None => continue,
            };
            if sector.contains_range(
                &(section_offset..section_offset + section_filesize),
            ) {
                println!("Matching section: {:?}", section.name()?);
                found = true;
            }
        }

        if !found {
            println!("No matching sections found!");
            continue;
        }

        extracted_data.push(DataChunk {
            address: p_paddr,
            segment_offset,
            segment_filesize,
        });
    }

    extracted_data.sort();

    let mut flash_data = Vec::new();
    for chunk in extracted_data {
        match flash_data.pop() {
            None => flash_data.push(chunk),
            Some(prev) => {
                let prev_len: u32 = prev.segment_filesize.try_into()?;
                let next_addr = prev.address + prev_len;
                if next_addr == chunk.address {
                    flash_data.push(DataChunk {
                        address: prev.address,
                        segment_offset: prev.segment_offset,
                        segment_filesize: prev.segment_filesize
                            + chunk.segment_filesize,
                    });
                } else {
                    flash_data.push(prev);
                    flash_data.push(chunk);
                }
            }
        }
    }

    Ok(flash_data)
}

fn main() -> Result<()> {
    let args = Args::parse();

    let work_dir = PathBuf::from(".");
    let path =
        build_artifact(&work_dir, &args.cargo_options.to_cargo_options())?
            .path()
            .to_owned();

    let _data = extract_data(&path)?;

    let probes = Probe::list_all();
    let _probe = probes[0].open()?;

    Ok(())
}
