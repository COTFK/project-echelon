# Project Echelon

A replay-to-video converter for [Project Ignis: EDOPro](https://projectignis.github.io/).

A [reference deployment] is available for testing. **Currently in beta - expect ongoing improvements!**

[reference deployment]: https://echelon.arqalite.org/

## Quick Start

### Using the Discord Bot (`@Echelon`)

Add the bot to your own server using this [install link](https://discord.com/oauth2/authorize?client_id=1452676046326595605).

1. **Send a replay** - DM or mention the bot with a `.yrpX` file attached
2. **Get queued** - Bot confirms your replay is queued with a unique ID
3. **Wait for processing** - Bot sends status updates as the replay is processed
4. **Download video** - Bot sends the finished MP4 when ready

### Using the Web UI

1. Visit the [web interface](https://echelon.arqalite.org/)
2. Upload a `.yrpX` replay file
3. Wait for processing and download your video

## Packages

The project consists of three independent services:

- **[echelon-server](packages/echelon-server)** - Core replay processing (Rust + Axum)
  - Handles replay validation and video encoding
  - Manages job queue and processing state
  - Requires: Xvfb, ffmpeg, EDOPro

- **[echelon-discord](packages/echelon-discord)** - Discord bot frontend (Rust + Serenity)
  - Accepts replay uploads via Discord DMs/mentions
  - Provides real-time status updates
  - Streams completed videos back to users

- **[echelon-webui](packages/echelon-webui)** - Web interface (Rust + Dioxus + Nginx)
  - Simple file upload interface
  - Real-time job status tracking
  - Video download link

## Local Development

### Prerequisites

- **Rust** 1.70+
- **Docker & Docker Compose** (for containerized setup)
- **Xvfb, ffmpeg** (if running server locally without Docker)

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

### Environment Variables

**Server:**

- `BOT_SECRET` - a secret key (can be any value, as long as you keep it secret!) used to identify requests coming from the Discord bot, and bypass the rate limit

**Discord Bot:**

- `DISCORD_TOKEN` - Discord bot authentication token (required)
- `ECHELON_SERVER_URL` - Server URL (defaults to `http://server:3000`)]
- `BOT_SECRET` - a secret key (can be any value, as long as you keep it secret!) to bypass server rate limits

**Web UI:**

- `API_BASE_URL` - Backend server URL (required for deployment)

### Docker Compose

All services use an `.env` file. Key variables:

```bash
DISCORD_TOKEN=your_bot_token_here
ECHELON_SERVER_URL=http://server:3000  # Internal Docker network
```

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

Additionally, prebuilt images for each service are available in the [Packages] tab, under their respective tags.

For example, to use the latest version of Echelon, run the following images:

- `project-echelon:server-latest`
- `project-echelon:webui-latest`
- `project-echelon:discord-latest`

[Packages]: https://git.arqalite.org/COTFK/-/packages/container/project-echelon/versions

## Contributing

We accept contributions! Submit your patches in the [Fire King Discord server](https://discord.gg/8JtxHUAdGq).

## License

Project Echelon is licensed under the [MIT license](LICENSE).

Unless explicitly stated otherwise, any contribution to this project shall be licensed under MIT, without any additional terms or conditions.
