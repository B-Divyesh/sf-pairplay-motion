# PairPlay Motion

PairPlay Motion turns two to four spare phones into motion controllers for a
shared browser screen. It is built for friends and families who want a quick
room game without installing an app, creating accounts, or pairing Bluetooth
hardware.

The free edition includes **Dead Still**. A one-time US $8 license unlocks
**News Desk** and **Ink Runner** through the Sociobot billing service. All three
games support real device motion and a keyboard/touch fallback.

## How it works

1. Open PairPlay on the shared display and choose **Host a game**.
2. Players scan the room QR or enter its six-character code on their phones.
3. The host requests calibration. Each phone explicitly grants motion access,
   or selects touch controls if its browser has no usable sensors.
4. The host starts a 40-second round and sees live scores.

Rooms and motion samples live only in server memory. SQLite holds one anonymous
aggregate page-view count per day—no IP, identity, or sensor history. Public
room creation and WebSocket upgrades use source and service-wide token buckets;
each connected relay is also bounded to a 30-message burst and 15 messages per
second. See
[`/privacy`](https://pairplay-motion.sociobot.in/privacy) for the product policy.

## Stack

- Svelte 5 + TypeScript + Vite PWA frontend
- Rust 2021, Axum WebSockets, Tokio, and SQLx/SQLite backend
- One non-root Alpine container serving the frontend and API on `PORT`
- Same-origin ephemeral WebSocket relay; no third-party runtime scripts or fonts

The visual and motion system is documented in [`.factory/design.md`](.factory/design.md).

## Local development

Requirements: Node 22+, npm, Rust 1.90+, and SQLite build dependencies.

```sh
npm ci
npm run build
cargo run
```

Open `http://localhost:8080`. To use physical phone sensors, serve the container
behind HTTPS (mobile browsers generally block sensors on insecure origins).
For split-device development, run the backend and Vite separately:

```sh
cargo run
npm run dev
```

Vite proxies `/api` and `/ws` to port 8080. Configuration is environment-only:

| Variable | Default | Purpose |
| --- | --- | --- |
| `PORT` | `8080` | HTTP/WebSocket listener |
| `DATABASE_URL` | `sqlite://data/pairplay.db?mode=rwc` | Aggregate counter database |
| `STATIC_DIR` | `dist` | Built frontend directory |
| `RUST_LOG` | application defaults | Structured log filter |
| `VITE_BILLING_BASE` | `https://api.sociobot.in` | Build-time Sociobot billing origin |

## Test and verify

```sh
npm test          # Vitest game rules + Rust route/unit tests
npm run build     # production frontend -> dist/
npm run test:e2e  # Chromium desktop/mobile, Axe, real multi-page WebSockets
npm audit
```

The Playwright version is pinned to match the factory-provided browsers. E2E
tests start the built Axum app automatically. No payment-provider card details
or secrets are stored in this repository.

## Container

```sh
docker build --build-arg BUILD_SHA="$(git rev-parse --short HEAD)" -t pairplay-motion .
docker run --rm -p 8080:8080 -v pairplay-data:/app/data pairplay-motion
```

The image runs as the unprivileged `pairplay` user. Deployment is handled by the
Param Factory; this repository does not manage DNS, billing registration, or
infrastructure. `BUILD_SHA` is required for container builds (and may not be
`container`), so `/health` always identifies the source of a released backend.

## Known browser limits

- iOS requires an explicit user gesture before its motion permission prompt.
- Sensor ranges and reporting rates differ on older phones.
- Controllers need a stable network route to the deployed host. The accessible
  touch/keyboard controls are available whenever motion is denied or absent.

## License

MIT. Generated hero artwork is original to this product; its prompt and
provenance are stored in `assets/src/`.
