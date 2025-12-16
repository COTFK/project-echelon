# Project Echelon - server

A single-threaded (for now) server that receives and converts [Project Ignis: EDOPro] replay files (`*.yrpX`) into videos.

It exposes three HTTP endpoints that clients can call to queue replays, check their status and download finished videos.

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

If you are comfortable using precompiled binaries, you can use the one provided in the `deployment/` directory (used for creating the Docker image used for our own deployment of Echelon).

Once that's done, create an .env file in this repository and set `EDOPRO_PATH` to point to your installation.

## Running the server

A `cargo run` should be all you need; the `axum` server will start listening at http://127.0.0.1:3000.


## API documentation
Three routes are available:

- `POST /upload`
    - `Content-Type: application/octet-stream`
    - The body of the request should be the `*.yrpX` file you want to convert (max 10 MB).
    - Uploads the replay file and queues it up for processing.
    - Returns:
        - `200 OK` with text body containing the ID: `01KAGRECJCNV1RESGHYAM7DZTB`
        - `400 Bad Request` if the file is not a valid `*.yrpX` file
        - `503 Service Unavailable` if the queue is full

- `GET /status/{id}`, where `id` is the ID received from `/upload`
    - Polls the queue for information on the given ID.
    - Returns an `application/json` response with the current status:
        - queued - `{"status":"queued", "position": 2}`
        - processing - `{"status":"processing"}`
        - done - `{"status":"done"}`
        - error - `{"status":"error", "message": "An error has occurred."}`
        - not found - `{"status":"not_found", "message": "..."}`

- `GET /download/{id}`, where `id` is the ID received from `/upload`
    - Requests the video file from the server.
    - If status is not `done` it will return a `404 Not Found`.
    - If status is `done`, it will return a `video/mp4` response with the video data.
    - **Note:** Successfully downloading a video removes it from the server.

## Docker deployments
Project Echelon is intended to be deployed in a Docker container; as such, a `server.Dockerfile` and `compose.yaml` representative of our reference deployment are provided in the repository root.

They should serve well as a starting point for your own deployment and should be ready to use - just `docker compose up --build` and you're good to go.

A prebuilt binary of our EDOPro fork alongside two config files (`system.conf` and `configs.json`) are provided in the `deployment/` directory of this package and are used by the Dockerfile to generate a ready-to-use image. Feel free to replace it with your own binaries/configs according to your needs.

## License
Project Echelon is licensed under the [MIT license](/COTFK/project-echelon/src/branch/master/LICENSE).

Unless explicitly stated otherwise, any contribution to this project shall be licensed under MIT, without any additional terms or conditions.