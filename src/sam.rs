use anyhow::Result;
use probe_rs::Probe;

pub struct Atsaml10(());

impl Atsaml10 {
    pub fn new() -> Self {
        Atsaml10(())
    }

    // XXX Unimplemented for now.
    pub fn erase(&self, probe: Probe) -> Result<Probe> {
        let interface = probe.try_into_arm_interface().map_err(|(_, e)| e)?;

        let interface = interface.initialize_unspecified()?;

        let probe = interface.close();

        Ok(probe)
    }
}
