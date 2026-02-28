# Digging

A monitoring server for [SimplifyDigging](https://github.com/Fatboychummy-CC/SimplifyDigging/tree/better) ComputerCraft turtle clients.

This Rust application provides a WebSocket server and REST API to track the status, position, and progress of mining turtles running the SimplifyDigging program.

## Features

- **WebSocket Server** - Real-time connection handling for turtle clients
- **REST API** - Query turtle status and data
- **Session Management** - Automatic timeout and cleanup of stale connections
- **Optional Metrics** - OpenTelemetry-compatible metrics instrumentation (via `metrics` feature flag)

## Quick Start with Docker

### Prerequisites

- Docker and Docker Compose installed on your system

### Running with Docker Compose

1. **Clone the repository:**
   ```bash
   git clone https://github.com/bananasov/digging
   cd digging
   ```

2. **Start the server:**
   ```bash
   docker compose up -d
   ```

3. **View logs:**
   ```bash
   docker compose logs -f digging
   ```

4. **Stop the server:**
   ```bash
   docker compose down
   ```

The server will be accessible at `http://localhost:3000`.

### Building with Metrics

To enable metrics instrumentation:

```bash
# Create environment file
cp .env.example .env

# Edit .env and set FEATURES=metrics
# Then build:
docker compose up --build -d

# Or inline:
FEATURES=metrics docker compose up --build -d
```

### Custom Configuration

Edit `.env` to customize:

- `BIND_ADDRESS` - Server bind address (default: `0.0.0.0:3000`)
- `HOST_PORT` - Port exposed on host machine (default: `3000`)
- `RUST_LOG` - Logging level (default: `digging=debug,tower_http=debug`)
- `FEATURES` - Build features to enable (set to `metrics` to enable metrics)

## API Endpoints

- `GET /` - Health check endpoint
- `GET /api/clients` - Get all connected clients and their data
- `GET /api/clients/{id}` - Get specific client data by session ID
- `WS /ws` - WebSocket endpoint for turtle connections

## Development Setup

### Prerequisites

- Rust 1.75 or later
- cargo

### Local Development

1. **Clone the repository:**
   ```bash
   git clone <your-repo-url>
   cd digging
   ```

2. **Run the development server:**
   ```bash
   cargo run
   ```

3. **Run with metrics enabled:**
   ```bash
   cargo run --features metrics
   ```

4. **Run tests:**
   ```bash
   cargo test
   ```

5. **Check code:**
   ```bash
   cargo check
   ```

The server will start on `127.0.0.1:3000` by default. To change the bind address:

```bash
BIND_ADDRESS=0.0.0.0:8080 cargo run
```

### Project Structure

```
digging/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library root
│   ├── metrics/             # Metrics instrumentation
│   ├── models/              # Data models
│   ├── routes/              # HTTP routes and WebSocket handlers
│   └── websockets/          # WebSocket session management
├── Dockerfile               # Multi-stage Docker build
├── docker-compose.yml       # Docker Compose configuration
└── Cargo.toml              # Rust dependencies
```

## Docker Build Options

### Build without metrics (smaller binary):
```bash
docker build -t digging:latest .
```

### Build with metrics:
```bash
docker build --build-arg FEATURES=metrics -t digging:metrics .
```

### Run with custom port:
```bash
docker run -p 8080:8080 -e BIND_ADDRESS=0.0.0.0:8080 digging:latest
```
