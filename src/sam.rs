use crate::elf::FlashData;
use anyhow::{anyhow, Result};
use probe_rs::architecture::arm::{ap::MemoryAp, ApAddress, DpAddress, Pins};
use probe_rs::{Memory, Probe};
use std::thread;
use std::time::Duration;

// I don't think it's possible to pass interface to a function,
// it seems to be a private type.
macro_rules! cpu_reset_extension {
    ($interface:ident) => {{
        let mut pin_out = Pins(0);
        let mut pin_mask = Pins(0);

        // 1 ms with reset high.
        pin_out.set_nreset(true);
        pin_mask.set_nreset(true);
        $interface.swj_pins(pin_out.0 as u32, pin_mask.0 as u32, 0)?;
        thread::sleep(Duration::from_millis(1));

        // 1 ms with reset low.
        pin_out.set_nreset(false);
        $interface.swj_pins(pin_out.0 as u32, pin_mask.0 as u32, 0)?;
        thread::sleep(Duration::from_millis(1));

        // 1 ms with reset and clock low.
        pin_mask.set_swclk_tck(true);
        $interface.swj_pins(pin_out.0 as u32, pin_mask.0 as u32, 0)?;
        thread::sleep(Duration::from_millis(1));

        // 1 ms with reset high.
        pin_mask.set_swclk_tck(false);
        pin_out.set_nreset(true);
        $interface.swj_pins(pin_out.0 as u32, pin_mask.0 as u32, 0)?;
        thread::sleep(Duration::from_millis(1));
    }};
}

pub struct Atsaml10(());

impl Atsaml10 {
    const DSU_ADDR: u32 = 0x41002100;
    const DSU_STATUSA_ADDR: u32 = Self::DSU_ADDR + 0x1;
    const DSU_STATUSB_ADDR: u32 = Self::DSU_ADDR + 0x2;
    const DSU_BCC0_ADDR: u32 = Self::DSU_ADDR + 0x20;
    const DSU_BCC1_ADDR: u32 = Self::DSU_ADDR + 0x24;
    const NVMCTRL_ADDR: u32 = 0x41004000;
    const NVMCTRL_CTRLA_ADDR: u32 = Self::NVMCTRL_ADDR + 0x00;
    const NVMCTRL_CTRLC_ADDR: u32 = Self::NVMCTRL_ADDR + 0x08;
    const NVMCTRL_STATUS_ADDR: u32 = Self::NVMCTRL_ADDR + 0x18;
    const NVMCTRL_ADDR_ADDR: u32 = Self::NVMCTRL_ADDR + 0x1C;
    // XXX Handle registers better.
    const CRSTEXT_BIT: u8 = 1 << 1;
    const BCCD1_BIT: u8 = 1 << 7;
    // Boot Interactive Mode commands (14.4.5.9).
    // Enter Interactive Mode.
    const CMD_INIT: u32 = 0x444247_55;
    // Exit Interactive Mode.
    const CMD_EXIT: u32 = 0x444247_AA;
    // ChipErease for SAM L10.
    const CMD_CHIPERASE: u32 = 0x444247_E3;
    // Boot Interactive Mode Status (14.4.5.10).
    // Debugger start communication.
    const SIG_COMM: u32 = 0xEC0000_20;
    // Dubber command success.
    const SIG_CMD_SUCCESS: u32 = 0xEC0000_21;
    // Valid command.
    const SIG_CMD_VALID: u32 = 0xEC0000_24;
    // Boot ROM ok to exit.
    const SIG_BOOTOK: u32 = 0xEC0000_39;
    // Flash row size.
    const ROW_SIZE: u32 = 256;
    // Erase row command.
    const NVMCTRL_CTRLA_ER_CMD: u16 = 0xa502;
    // XXX Overwrite the first two bytes of CTRLB which default to 0...
    const NVMCTRL_CTRLA_ER_CMD32: u32 =
        (Self::NVMCTRL_CTRLA_ER_CMD as u32) << 16;
    const NVMCTRL_STATUS_READY: u8 = 1 << 2;

    pub fn new() -> Self {
        Atsaml10(())
    }

    pub fn erase(&self, probe: Probe) -> Result<Probe> {
        let mut interface =
            probe.try_into_arm_interface().map_err(|(_, e)| e)?;

        cpu_reset_extension!(interface);

        let mut interface = interface.initialize_unspecified()?;

        let port = ApAddress {
            dp: DpAddress::Default,
            ap: 0,
        };

        let default_memory_ap = MemoryAp::new(port);
        {
            let mut memory = interface.memory_interface(default_memory_ap)?;

            self.exit_reset_extension(&mut memory)?;

            log::warn!("XXXa4");

            // Request Boot ROM Interactive mode entry (14.4.5.1.1).
            memory
                .write_word_32((Self::DSU_BCC0_ADDR).into(), Self::CMD_INIT)?;

            log::warn!("XXXa5");

            // Check for SIG_COMM status in DSU.BCC1.
            let status = memory.read_word_32((Self::DSU_BCC1_ADDR).into())?;
            // Possibly I need to wait for the bit to be set?
            if status != Self::SIG_COMM {
                log::warn!("XXX status wrong: {:x}", status);
                return Err(anyhow!(
                    "Failed to enter Boot ROM interactive mode."
                ));
            }

            log::warn!("XXXa6");

            // Issue the Chip Erase command (14.4.5.4.1).
            memory.write_word_32(
                (Self::DSU_BCC0_ADDR).into(),
                Self::CMD_CHIPERASE,
            )?;

            // Check to see if the command was valid.
            let status = memory.read_word_32((Self::DSU_BCC1_ADDR).into())?;
            if status != Self::SIG_CMD_VALID {
                log::warn!("XXX status wrong: {:x}", status);
                return Err(anyhow!(
                    "Chip Erase failed due to invalid command"
                ));
            }

            log::warn!("XXXa7");

            // Poll for status update.
            let mut status = 0;
            for i in 0..20 {
                status = memory.read_word_32((Self::DSU_BCC1_ADDR).into())?;
                if status != Self::SIG_CMD_VALID && status != 0 {
                    // XXX Change this to trace.
                    log::warn!("Received status update after {} cycles", i);
                    break;
                }
                // No status update, wait for a while before trying again.
                thread::sleep(Duration::from_secs(1));
            }

            log::warn!("XXXa8");

            // Make sure we were successful.
            if status != Self::SIG_CMD_SUCCESS {
                // XXX is warn the right message?
                log::warn!("XXX Chip Erase failed!");
                // XXX reset to park?
            } else {
                // XXX warn?
                log::warn!("XXX Chip Erase succeeded");
            }

            log::warn!("XXXa9");
        }

        let probe = interface.close();

        Ok(probe)
    }

    pub fn program(&self, probe: Probe, data: &FlashData) -> Result<Probe> {
        let mut interface =
            probe.try_into_arm_interface().map_err(|(_, e)| e)?;

        cpu_reset_extension!(interface);

        let mut interface = interface.initialize_unspecified()?;

        let port = ApAddress {
            dp: DpAddress::Default,
            ap: 0,
        };

        let default_memory_ap = MemoryAp::new(port);
        {
            let mut memory = interface.memory_interface(default_memory_ap)?;

            self.exit_reset_extension(&mut memory)?;

            log::warn!("XXXa9d");

            // XXX Not sure I'm doing this right.
            memory
                .write_word_32((Self::DSU_BCC0_ADDR).into(), Self::CMD_EXIT)?;

            log::warn!("XXXaa");

            // Poll for status update.
            for _ in 0..20 {
                let statusb =
                    memory.read_word_8((Self::DSU_STATUSB_ADDR).into())?;
                if (statusb & Self::BCCD1_BIT) != 0 {
                    let status =
                        memory.read_word_32((Self::DSU_BCC1_ADDR).into())?;
                    if status != Self::SIG_BOOTOK {
                        log::warn!(
                            "Failed to exit to park!: status {:x}",
                            status
                        );
                        // XXX Error!
                    }
                }
                // No status update, wait for a while before trying again.
                thread::sleep(Duration::from_millis(50));
            }

            log::warn!("Exit to park succeeded!");

            let row_size: usize = Self::ROW_SIZE as usize;

            // Actually do the flash.
            log::warn!("Flashing!");
            for chunk in &data.chunks {
                let data = &data.bin_data[chunk.segment_offset as usize..]
                    [..chunk.segment_filesize as usize];

                // Enable automatic writes.
                memory.write_word_8((Self::NVMCTRL_CTRLC_ADDR).into(), 0)?;

                let mut addr = chunk.address;

                for row in data.chunks(row_size) {
                    // Set the address.
                    memory.write_word_32(
                        (Self::NVMCTRL_ADDR_ADDR).into(),
                        addr,
                    )?;

                    // Clear memory row.
                    // XXX Would prefer to write this as a 16 bit addr...
                    memory.write_word_32(
                        (Self::NVMCTRL_CTRLA_ADDR).into(),
                        Self::NVMCTRL_CTRLA_ER_CMD32,
                    )?;

                    // Wait for the NVM controller to be ready.
                    loop {
                        let status = memory
                            .read_word_8((Self::NVMCTRL_STATUS_ADDR).into())?;
                        if (status & Self::NVMCTRL_STATUS_READY) != 0 {
                            break;
                        }
                    }

                    if row.len() < row_size {
                        let mut buf = Vec::with_capacity(row_size);
                        buf.extend_from_slice(row);
                        buf.resize(row_size, 0xff);

                        memory.write_8(addr.into(), &buf)?;
                    } else {
                        memory.write_8(addr.into(), row)?;
                    }

                    addr += Atsaml10::ROW_SIZE;
                }
            }
        }

        let probe = interface.close();

        Ok(probe)
    }

    pub fn verify(&self, probe: Probe, data: &FlashData) -> Result<Probe> {
        let mut interface =
            probe.try_into_arm_interface().map_err(|(_, e)| e)?;

        cpu_reset_extension!(interface);

        let mut interface = interface.initialize_unspecified()?;

        let port = ApAddress {
            dp: DpAddress::Default,
            ap: 0,
        };

        let default_memory_ap = MemoryAp::new(port);
        {
            let mut memory = interface.memory_interface(default_memory_ap)?;

            self.exit_reset_extension(&mut memory)?;

            log::warn!("XXXa9d");

            // XXX Not sure I'm doing this right.
            memory
                .write_word_32((Self::DSU_BCC0_ADDR).into(), Self::CMD_EXIT)?;

            log::warn!("XXXaa");

            // Poll for status update.
            for _ in 0..20 {
                let statusb =
                    memory.read_word_8((Self::DSU_STATUSB_ADDR).into())?;
                if (statusb & Self::BCCD1_BIT) != 0 {
                    let status =
                        memory.read_word_32((Self::DSU_BCC1_ADDR).into())?;
                    if status != Self::SIG_BOOTOK {
                        log::warn!(
                            "Failed to exit to park!: status {:x}",
                            status
                        );
                        // XXX Error!
                    }
                }
                // No status update, wait for a while before trying again.
                thread::sleep(Duration::from_millis(50));
            }

            log::warn!("Exit to park succeeded!");

            let row_size: usize = Self::ROW_SIZE as usize;

            // Verify the data.
            log::warn!("Verifying");
            let mut read_data = Vec::with_capacity(row_size);
            read_data.resize(row_size, 0xff);

            'chunks: for chunk in &data.chunks {
                let data = &data.bin_data[chunk.segment_offset as usize..]
                    [..chunk.segment_filesize as usize];

                let mut addr = chunk.address;

                for row in data.chunks(row_size) {
                    memory.read_8(addr.into(), &mut read_data)?;
                    for ((i, expected), actual) in
                        row.iter().enumerate().zip(read_data.iter())
                    {
                        if expected != actual {
                            println!(
                                "Values at address {:x} don't match \
                                 ({:x} != {:x}",
                                addr as usize + i,
                                actual,
                                expected
                            );
                            // XXX Find a way to error here.
                            break 'chunks;
                        }

                        addr += Atsaml10::ROW_SIZE;
                    }
                }
            }
        }

        let probe = interface.close();

        Ok(probe)
    }

    pub fn reset(&self, probe: Probe) -> Result<()> {
        let mut interface =
            probe.try_into_arm_interface().map_err(|(_, e)| e)?;

        let mut pin_out = Pins(0);
        let mut pin_mask = Pins(0);

        // Make sure the SWCLK pin is high so we don't enter cold plug.

        // Enter reset for 2 ms.
        pin_out.set_nreset(false);
        pin_out.set_swclk_tck(true);
        pin_mask.set_nreset(true);
        pin_mask.set_swclk_tck(true);
        interface.swj_pins(pin_out.0 as u32, pin_mask.0 as u32, 0)?;
        thread::sleep(Duration::from_millis(2));

        // Clear reset.
        pin_out.set_nreset(true);
        interface.swj_pins(pin_out.0 as u32, pin_mask.0 as u32, 0)?;

        Ok(())
    }

    fn exit_reset_extension(&self, memory: &mut Memory) -> Result<()> {
        // Make sure the CRSTEXT bit is set to indicate we're in the
        // reset extension phase.
        let statusa = memory.read_word_8((Self::DSU_STATUSA_ADDR).into())?;
        if (statusa & Self::CRSTEXT_BIT) == 0 {
            // XXX Better warning message?
            log::warn!("Reset extension failed, need `--connect-under-reset`?");
            return Err(anyhow!("CPU Reset Extension failed"));
        }

        log::warn!("XXXa1");

        // Clear the CRSTEXT bit.
        memory
            .write_word_8((Self::DSU_STATUSA_ADDR).into(), Self::CRSTEXT_BIT)?;

        log::warn!("XXXa2");

        // Wait 5ms for CPU to execute Boot ROM failure analysis and security
        // checks.
        thread::sleep(Duration::from_millis(5));

        log::warn!("XXXa3");

        // Check to see if there were any errors.
        let statusb = memory.read_word_8((Self::DSU_STATUSB_ADDR).into())?;
        if (statusb & Self::BCCD1_BIT) != 0 {
            log::warn!("Boot discovered errors, continuing: XXX");
            // XXX Go read the error code and show to the user.
        }

        Ok(())
    }
}
