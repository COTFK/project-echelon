# Project Echelon
A replay-to-video converter for [Project Ignis: EDOPro](https://projectignis.github.io/).

A [reference deployment] is available for testing. **Currently a very rough WIP - expect jank and breakage!**

[reference deployment]: https://echelon.arqalite.org/

## Packages
The project currently maintains two packages:

- [echelon-server](packages/echelon-server) - a Rust+Axum server that handles the conversion
- [echelon-webui](packages/echelon-webui) - a web UI (in Rust+Dioxus) to interface with the server

## License
Project Echelon is licensed under the [MIT license](LICENSE).

Unless explicitly stated otherwise, any contribution to this project shall be licensed under MIT, without any additional terms or conditions.