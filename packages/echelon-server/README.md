# Project Echelon - server

A single-threaded (for now) server that receives and converts [Project Ignis: EDOPro] replay files (`*.yrpX`) into videos.

It exposes HTTP endpoints that clients can call to queue replays, check their status and download finished videos.

## Requirements
- A Linux distribution with X11 (Wayland is not supported at the moment)
- [Rust] 1.90.0 or higher
- [xvfb]
- [ffmpeg] w/ libx264
- [Our fork of Project Ignis: EDOPro] (see the Preparing EDOPro section for details)

[Rust]: https://rust-lang.org/
[xvfb]: https://x.org/releases/X11R7.7/doc/man/man1/Xvfb.1.xhtml
[ffmpeg]: https://www.ffmpeg.org/
[Our fork of Project Ignis: EDOPro]: https://git.arqalite.org/COTFK/project-echelon-edopro

## Preparing EDOPro
You will need to set up a working EDOPro distribution before getting started. 

The easiest way to do so is to install the [official build](https://projectignis.github.io/download.html), then overwriting the `EDOPro` executable with the one compiled from [our fork](https://git.arqalite.org/COTFK/project-echelon-edopro).

Alternatively, we recommend using Docker to run the server, see the "Docker deployments" below - this will set up and compile EDOPro for you.

Once that's done, create an .env file in this repository and set `EDOPRO_PATH` to point to your installation.

## Running the server

A `cargo run` should be all you need; the `axum` server will start listening at http://127.0.0.1:3000.

## API documentation
The API is documented using [Bruno](https://www.usebruno.com/), in the `api` folder of the repository root.

## Docker deployments
Project Echelon is intended to be deployed in a Docker container; as such, a `server.Dockerfile` and `compose.yaml` representative of our reference deployment are provided in the repository root.

They should serve well as a starting point for your own deployment and should be ready to use - just `docker compose up --build` and you're good to go.

## License
Project Echelon is licensed under the [MIT license](/COTFK/project-echelon/src/branch/master/LICENSE).

Unless explicitly stated otherwise, any contribution to this project shall be licensed under MIT, without any additional terms or conditions.