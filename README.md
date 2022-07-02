# cargo-saml10

This is a hack to create a cargo subcommand to program Microchip
ATSAML10 chips based on [probe-rs][probe-rs].

## Usage

```sh
$ cargo saml10
    Finished dev [unoptimized + debuginfo] target(s) in 0.02s
Programming /tmp/rust_on_arm/saml10-test/target/thumbv8m.base-none-eabi/debug/blink
Erasing...Done
Flashing.......................Done
Verifying.......................Done
```

## Status

This works for me enough to program a super basic blink program onto a
[SAML10 XPLAINED][xplained] eval board using a [MAX32625PICO][pico]
connected to the external debug port.

I haven't tested anything more that that at this point. My hope is to
integrate the logic into [probe-rs][probe-rs] so that it works with
[cargo-flash][cargo-flash] and [cargo-embed][cargo-embed].

## Inspiration

 - [edbg][edbg] is a programmer that works with the onboard programmer
   of the [XPLAINED][xplained] board, but doesn't automatically work
   with the [MAX32625PICO][pico] programmer (I haven't tried to debug
   this). Rust generated ELF files have to be converted to bin files
   before using this tool.
 - [MPLAB IPE][ipe] will also let you program using the built in
   debugger as long as you convert the ELF files to hex files first.

## License

Choose any of:

 - Apache 2.0: [LICENSE-APACHE](LICENSE-APACHE)
 - MIT: [LICENSE-MIT](LICENSE-MIT)
 - Blue Oak: [LICENSE-BLUE-OAK.md](LICENSE-BLUE-OAK.md)

[xplained]: https://www.microchip.com/en-us/development-tool/DM320204
[pico]: https://www.maximintegrated.com/en/products/microcontrollers/MAX32625PICO.html
[probe-rs]: https://probe.rs/
[cargo-flash]: https://github.com/probe-rs/cargo-flash
[cargo-embed]: https://github.com/probe-rs/cargo-embed
[edbg]: https://github.com/ataradov/edbg
[ipe]: https://www.microchip.com/en-us/tools-resources/production/mplab-integrated-programming-environment
