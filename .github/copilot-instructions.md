# Project Echelon — Copilot Instructions

## Architecture

Three independent Rust crates in a workspace (`packages/`), communicating over HTTP:

- **echelon-server** — Axum HTTP API that queues `.yrpX` replay files, drives a headless EDOPro instance via Xvfb + ffmpeg to render MP4 videos, and serves them for download. This is the core service; everything else is a thin client.
- **echelon-discord** — Serenity-based Discord bot that accepts replay uploads via slash commands, polls the server for status, and delivers finished videos to users.
- **echelon-webui** — Dioxus 0.7 WASM frontend served by nginx. Uses Tailwind CSS + DaisyUI ("business" theme). Compiled with `dx bundle`.

Data flow: Client uploads `.yrpX` → Server queues job (ULID ID) → Background worker runs EDOPro in Xvfb, captures frames/audio via named FIFOs, encodes with ffmpeg → Client polls `/status/{id}` → Client downloads `/download/{id}`.

## Key Conventions

- **Rust 2024 edition** (resolver v3). Minimum Rust version: 1.90.0.
- **Job IDs**: ULID (`ulid` crate) — monotonically increasing, embeds timestamp (used by cleanup for TTL). Generated via `Ulid::new()`.
- **Shared state**: `Arc<RwLock<BTreeMap<Ulid, Replay>>>` — `BTreeMap` ensures ordered iteration so the worker always picks the oldest queued job. Video data is held **in memory** until cleanup.
- **Error handling**: `anyhow::Result` in worker/commands (binary-only modules); typed `ReplayError` enums in `types.rs` for route-level validation. The discord crate uses `Result<_, String>` for simplicity.
- **Logging**: `tracing` with `tracing_subscriber::fmt`. Prefix log lines with `[{ulid}]` for job correlation. Use `info` for lifecycle, `debug` for internals, `warn` for rate limits, `error` for failures.
- **Environment config**: `dotenvy` for `.env` files. Key vars: `EDOPRO_PATH`, `BOT_SECRET`, `DISCORD_TOKEN`, `ECHELON_SERVER_URL`, `API_BASE_URL` (compile-time for webui via `option_env!`).

## Server API (echelon-server)

Two-step upload flow:

1. `POST /create` — JSON body `{"top_down_view": bool}`, returns ULID. Creates job in `Created` status.
2. `POST /upload?task_id={ulid}` — binary body (the `.yrpX` file). Validates magic bytes (`b"yrpX"`), parses replay packets for duration estimation, transitions to `Queued`.

All clients (webui, discord bot) must use this two-step flow.

Other routes: `GET /status/{id}` (JSON status), `GET /download/{id}` (MP4 with Range support, `?download=1` for attachment), `GET /health`.

Rate limiting: `tower_governor` GCRA, 5 req/60s per IP. Bot bypasses via `X-Bot-Secret` header.

## Module Structure

**echelon-server** has dual targets: `main.rs` (binary) owns `commands` and `worker` modules (not exported); `lib.rs` exports `estimation`, `routes`, `types` for integration tests.

```
echelon-server/src/
├── main.rs        # Server startup, routing, rate limiting, graceful shutdown
├── lib.rs         # Re-exports for tests: estimation, routes, types
├── commands.rs    # Process spawning: EDOPro, Xvfb, PulseAudio, ffmpeg (binary-only)
├── estimation.rs  # yrpX binary parser + duration estimator
├── routes.rs      # HTTP handlers: create, upload, status, download
├── types.rs       # Replay, ReplayStatus, ReplayConfig, ReplayError
└── worker.rs      # Background job processor + cleanup loop (binary-only)
```

**echelon-discord**: `main.rs` (bot event loop, slash command handler, `monitor_replay` polling loop), `api.rs` (HTTP client for server), `lib.rs` (re-exports `api` for tests).

**echelon-webui**: `main.rs` (Dioxus Router: `/`, `/privacy`, `/terms`), `api.rs` (ApiClient wrapping reqwest), `types.rs` (client-side ReplayStatus with extra `Idle`/`Uploading` states), `components/` (navbar, footer, hero, upload_form, status_display, video_preview, privacy_policy, terms_of_service).

## Building & Testing

```bash
# Run all tests (11 discord + 27 server)
cargo test --all

# Run a specific package
cargo run --package echelon-server
cargo run --package echelon-discord

# Web UI (requires Dioxus CLI + wasm32-unknown-unknown target)
cd packages/echelon-webui && dx serve

# Full system via Docker
docker compose up --build        # Server :3000, WebUI :8080
docker compose watch             # Auto-rebuild on src/ changes
```

Server integration tests use `axum_test::TestServer` with a rate-limit-free app variant (`create_app_without_rate_limit`). Discord tests use `mockito` to mock the server HTTP API.

## EDOPro Fork (`packages/echelon-edopro/`)

A custom fork of [edo9300/edopro](https://github.com/edo9300/edopro) modified for headless offline replay rendering. **Not intended for interactive play.** Three key changes vs upstream:

1. **Frame capture** (`gframe/game.cpp`) — When `EDOPRO_OFFLINE_RENDER=1`, time is simulated at `1000/60` ms per tick (deterministic 60fps, not wall-clock). When `EDOPRO_FRAME_PIPE` is set, each frame is rendered to a texture, cropped via `EDOPRO_FRAME_CROP_X/Y/W/H` env vars, and `fwrite()`d as raw BGRA to the named FIFO. Capture is only active during replay duel playback.
2. **Offline audio mixer** (`gframe/offline_audio_mixer.h/.cpp`) — Entirely new singleton that decodes audio files into float32 PCM, mixes active sounds per tick via `MixForMillis()`, converts to s16le stereo 44100 Hz, and writes to the `EDOPRO_AUDIO_PIPE` FIFO.
3. **SoundManager redirect** (`gframe/sound_manager.cpp`) — When `offlineRender` is true, all `PlaySoundEffect()`/`PlayBGM()`/`PlayChant()` calls route to `OfflineAudioMixer` instead of the normal sound backend.

Built with **premake5** → `make` (target: `ygoprodll`). Build scripts in `travis/`: `dependencies.sh` (fetches vcpkg cache), `install-premake5.sh`, `build.sh` (runs premake5 + make release).

## EDOPro Integration (server only)

The server orchestrates the EDOPro fork. Key details in `commands.rs`:
- Xvfb runs on display `:99` at 1556x1000x24; PulseAudio uses a null sink on a UNIX socket (needed for EDOPro audio subsystem init, even though actual audio goes through the FIFO).
- Server creates two named FIFOs (`frames.pipe`, `audio.pipe`) via `mkfifo()`, then launches EDOPro with env vars pointing to them plus crop config (`EDOPRO_FRAME_CROP_X=456`).
- ffmpeg reads raw BGRA from the frame pipe (1100x1000 @ 60fps, skips first 10 frames) → H.264 veryfast → video-only MP4.
- A Tokio task reads raw PCM from the audio pipe into a file. After EDOPro exits, video and audio are muxed: `ffmpeg -c:v copy -c:a aac -movflags +faststart`.
- Job timeout: 1 hour. ffmpeg shutdown timeout: 30 seconds (SIGTERM if exceeded).

## Docker

Each service has its own Dockerfile in the repo root. All use `cargo-chef` for dependency caching. The server Dockerfile has a 4th stage that compiles the EDOPro C++ fork from `packages/echelon-edopro/` using GCC 15. Deployment config/textures live in `packages/echelon-server/deployment/`.
