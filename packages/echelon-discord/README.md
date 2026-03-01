# Project Echelon - Discord bot

A Discord bot that interfaces with [echelon-server](/packages/echelon-server) and allows users to submit replay files (`*.yrpX`) directly through Discord DMs or mentions.

## Features
- Receive replay files via Discord DMs or channel mentions
- Automatic upload to echelon-server
- Real-time status updates (queued position, processing status)
- Automatic video download and delivery when ready
- Custom presence status

## Requirements
- [Rust] 1.90.0 or higher
- A Discord bot token from the [Developer Portal]

[Rust]: https://rust-lang.org/
[Developer Portal]: https://discord.com/developers/applications

## Setup

### Create a Discord Bot
1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application" and give it a name
3. Go to the "Bot" section and click "Add Bot"
4. Under the TOKEN section, click "Copy" to copy your bot token

### Configure Environment Variables
Create a `.env` file in this directory:
```bash
DISCORD_TOKEN=your_bot_token_here
ECHELON_SERVER_URL=http://localhost:3000  # Point it to your instance of echelon-server
```

### Invite the Bot to Your Server
1. In Developer Portal, go to OAuth2 → URL Generator
2. Select scopes: `bot`
3. Select permissions: `Send Messages`
4. Copy the generated URL and open it in your browser to invite the bot

## Running the Bot

```bash
cargo run --package echelon-discord
```

The bot will connect to Discord and display its status when online.

## Testing
For testing with a separate bot instance:
1. Create a second bot in the Developer Portal
2. Create a test `.env` file or use a different token
3. Run with a different token to test without affecting the production bot

## How to Use
Call the `/echelon convert` command and attach your replay file.

The bot will:
1. Acknowledge receipt with a replay ID
2. Show the queue position
3. Update you when processing starts
4. Send the finished video when ready

## Docker Deployments
Project Echelon is intended to be deployed in a Docker container; as such, a `echelon.Dockerfile` and `compose.yaml` representative of our reference deployment are provided in the repository root.

They should serve well as a starting point for your own deployment and should be ready to use - just `docker compose up --build` and you're good to go.

## Architecture
- **main.rs** - Discord event handling, replay monitoring, and status updates
- **api.rs** - Server API communication (upload, status checks, video downloads)

## License
Project Echelon is licensed under the [MIT license](/COTFK/project-echelon/src/branch/master/LICENSE).

Unless explicitly stated otherwise, any contribution to this project shall be licensed under MIT, without any additional terms or conditions.
