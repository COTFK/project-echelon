# Project Echelon

A replay-to-video converter for [Project Ignis: EDOPro](https://projectignis.github.io/).

## Quick Start

### Using the Discord Bot (`@Echelon`)

Add the bot to your own server using this [install link](https://discord.com/oauth2/authorize?client_id=1452676046326595605).

1. **Send a replay** - call the `/echelon convert` command and attach your replay file
2. **Get queued** - Echelon confirms your replay is queued
3. **Wait for processing** - You'll receive status updates as the replay is processed
4. **Download video** - Echelon sends the finished MP4 when ready

### Using the Web UI

1. Visit the [web interface](https://echelon.arqalite.org/)
2. Upload a `.yrpX` replay file
3. Wait for processing and download your video

## Packages

The project consists of three independent services and a custom fork of EDOPro:

- **[echelon-server](packages/echelon-server)** - Core replay processing (Rust + Axum)
  - Handles replay validation and video encoding
  - Manages job queue and processing state
  - Requires: Xvfb, ffmpeg, oEDOPro

- **[echelon-discord](packages/echelon-discord)** - Discord bot frontend (Rust + Serenity)
  - Accepts replay uploads via Discord DMs/mentions
  - Provides real-time status updates
  - Streams completed videos back to users

- **[echelon-webui](packages/echelon-webui)** - Web interface (Rust + Dioxus + Nginx)
  - Simple file upload interface
  - Real-time job status tracking
  - Video download link
  
- **[echelon-edopro](https://github.com/COTFK/project-echelon-edopro)** - custom fork of EDOPro (C++)
  - Added offline rendering to audio and video FIFO pipes, for perfectly smooth 60fps video output
  - Adjusted UI elements to better fit a video recording
  - Added command line arguments and environment variables to configure EDOPro

## Local Development

### Prerequisites

- **Rust** 1.70+
- **Docker & Docker Compose** (for containerized setup)
- **Xvfb, ffmpeg** (if running server locally without Docker)
- Optionally, for `echelon-edopro`, a C++ development environment
- For the Discord bot, you will need to create an application in the Discord Developer Portal, and obtain your token (DISCORD_TOKEN in our environment)

### Running the entire system with Docker Compose

```bash
# Copy example env file and configure
cp .env.example .env
# Edit .env with your DISCORD_TOKEN and other settings

# Start all services
docker compose up --build

# Services available at:
# - Server: http://localhost:3000
# - Web UI: http://localhost:8080
# - Discord: Invite the bot to your server
```

For running the packages individually, check their respective `README.md` files.

## Configuration

Project Echelon is configured exclusively via environment variables. A `.env.example` file is provided in the repository root, listing all the environment variables and example values where applicable.

## Testing

Run all tests:

```bash
cargo test --all
```

Tests include:

- **API tests** (11 tests) - Discord bot server communication
- **Server tests** (27 tests) - Replay validation, job management, API routes

## Deployment

Project Echelon is intended to be deployed in a Docker environment; as such, Dockerfiles and a `compose.yaml` representative of our reference deployment are provided in the repository root.

They should serve well as a starting point for your own deployment and should be ready to use - just `docker compose up --build` and you're good to go.

## Contributing

We accept contributions! Reach out to us in the [Fire King Discord server](https://discord.gg/8JtxHUAdGq) to get access!

## License

Project Echelon is licensed under the [MIT license](LICENSE).

Project Ignis: EDOPro, and the custom fork in this repository, are licensed under [the AGPL-3.0 license](packages/echelon-edopro/LICENSE).

Unless explicitly stated otherwise, any contribution to this project shall be licensed under the aforementioned licenses, without any additional terms or conditions.
