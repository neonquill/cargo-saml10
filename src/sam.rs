use anyhow::Result;
use probe_rs::architecture::arm::Pins;
use probe_rs::Probe;
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
    pub fn new() -> Self {
        Atsaml10(())
    }

    // XXX Unimplemented for now.
    pub fn erase(&self, probe: Probe) -> Result<Probe> {
        let mut interface =
            probe.try_into_arm_interface().map_err(|(_, e)| e)?;

        cpu_reset_extension!(interface);

        let interface = interface.initialize_unspecified()?;

        let probe = interface.close();

        Ok(probe)
    }
}
