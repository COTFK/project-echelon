# Project Echelon - Web UI

A web application that interfaces with [echelon-server](/packages/echelon-server) and provides an easy-to-use UI for users to convert their replays with.

## Requirements
- [Rust] 1.90.0 or higher
- The [Dioxus CLI]

[Rust]: https://rust-lang.org/
[Dioxus CLI]: https://dioxuslabs.com/learn/0.7/getting_started/#install-the-dioxus-cli

## Building and testing
With Dioxus CLI installed, run `dx serve` - the website should be accessible at `http://127.0.0.1:8080`.

## Docker deployments
Project Echelon is intended to be deployed in a Docker container; as such, a `webui.Dockerfile` and `compose.yaml` representative of our reference deployment are provided in the repository root.

They should serve well as a starting point for your own deployment and should be ready to use - just `docker compose up --build` and you're good to go.

## License
Project Echelon is licensed under the [MIT license](/COTFK/project-echelon/src/branch/master/LICENSE).

Unless explicitly stated otherwise, any contribution to this project shall be licensed under MIT, without any additional terms or conditions.