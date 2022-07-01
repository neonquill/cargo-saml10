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
    // XXX Handle registers better.
    const CRSTEXT_BIT: u8 = 1 << 1;
    const BCCD1_BIT: u8 = 1 << 7;
    // Boot Interactive Mode commands (14.4.5.9).
    // Enter Interactive Mode.
    const _CMD_INIT: u32 = 0x444247_55;
    // Exit Interactive Mode.
    const _CMD_EXIT: u32 = 0x444247_AA;
    // ChipErease for SAM L10.
    const _CMD_CHIPERASE: u32 = 0x444247_E3;
    // Boot Interactive Mode Status (14.4.5.10).
    const _NVMCTRL_STATUS_READY: u8 = 1 << 2;

    pub fn new() -> Self {
        Atsaml10(())
    }

    // XXX Unimplemented for now.
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
        }
        let probe = interface.close();

        Ok(probe)
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

        // XXX Still need to actually run the erase command.

        log::warn!("XXXa4");

        Ok(())
    }
}
