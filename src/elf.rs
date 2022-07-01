use anyhow::{anyhow, Result};
use object::elf::{FileHeader32, PT_LOAD};
use object::read::elf::ProgramHeader;
use object::{Endianness, Object, ObjectSection};
use probe_rs::config::MemoryRange;
use std::path::Path;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct DataChunk {
    pub address: u32,
    pub segment_offset: u64,
    pub segment_filesize: u64,
}

pub struct FlashData {
    pub bin_data: Vec<u8>,
    pub chunks: Vec<DataChunk>,
}

pub fn extract_data(path: &Path) -> Result<FlashData> {
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

        log::info!(
            "Loadable Segment physical {:x}, virtual {:x}",
            p_paddr,
            p_vaddr
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
                log::info!("Found matching section: {:?}", section.name()?);
                found = true;
            }
        }

        if !found {
            log::warn!("No matching sections found!");
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

    Ok(FlashData {
        bin_data,
        chunks: flash_data,
    })
}
