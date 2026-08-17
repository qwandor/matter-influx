# matter-influx

[![crates.io page](https://img.shields.io/crates/v/matter-influx.svg)](https://crates.io/crates/matter-influx)

matter-influx is a Matter controller which logs sensor values to InfluxDB. It also provides a basic
local web dashboard to view the current values.

## Installation

The recommended way to install matter-influx is from the Debian package. The latest release can be
found on the [GitHub releases page](https://github.com/qwandor/matter-influx/releases).

You can also build it yourself with `cargo deb`. In the root of this repository:

```sh
$ cargo install cargo-deb
$ cargo deb
$ dpkg -i target/debian/matter-influx_*.deb
```

## Usage

Install the package on a Rasberry Pi or other always-on machine on your local network. Edit
`/etc/matter-influx.toml` to configure it to your liking. You'll need to restart the `matter-influx`
service after editing the config for it to take effect. You should then be able to open
http://raspberry-pi.lan:3009/ (or whatever hostname the machine you installed it on has) in a web
browser to connect some Matter devices.

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the
work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
