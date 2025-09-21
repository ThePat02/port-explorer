[![Rust Build](https://github.com/SpiceSniper/port-explorer/actions/workflows/rust.yml/badge.svg)](https://github.com/SpiceSniper/port-explorer/actions/workflows/rust.yml)
# Port Explorer
Port Explorer is a fast network port and service discovery written entirely in Rust!

## Features
- High Performance TCP Port Scanning
- Service Recognition through HTML Header Parsing
- Configurability through config file
- Pluggable signature rules (YAML)
- **NEW:** Web-based GUI for easy configuration and real-time results
- **NEW:** Real-time progress tracking with WebSocket updates
- **NEW:** Modern responsive interface for all screen sizes

## Getting Started
### Prerequisites
- [Rust toolchain (stable)](https://www.rust-lang.org/tools/install)
- Linux(tested)/macOS(untested)/Windows(untested)

### Build
```sh
cargo build --release
```

### Run

#### Command Line Interface (CLI)
```sh
./target/release/port-explorer <config_path>
```
You can run the program by executing the shell command above. You can optionally pass the path to a config file, if no path is passed or the path is invalid `./config.yaml` is used.

#### Graphical User Interface (GUI)
```sh
./target/release/port-explorer --gui [port]
```
Starts the web-based GUI server. The optional port parameter specifies which port the web server listens on (default: 3030). Access the interface by opening `http://localhost:3030` in your web browser.

**GUI Features:**
- Interactive form for scan configuration
- Real-time progress tracking with visual progress bars
- Live results display with port and service information  
- Responsive design that works on desktop and mobile
- WebSocket-based real-time updates during scanning

### Configuration
Edit `config.yaml` (or a config file of your choice) to set scan parameters:
- `ip`: Target IP address
- `start_port`, `end_port`: Port range
- `max_threads`: Concurrency
- `language`: Localization (e.g., `en` -> filename with out `.yaml`)

Signatures for service identification are in `signatures/` (YAML files). You can add new yaml files and subfolders into the `signatures/` folder, as it gets parsed recursively. 


## Usage

### CLI Mode
- Run a scan: `./target/release/port-explorer <config_path>`
- Logs are written to `logs/` with timestamped filenames
- Localization files in `resources/Localization/`

### GUI Mode  
- Start the GUI: `./target/release/port-explorer --gui [port]`
- Open web browser to `http://localhost:3030` (or specified port)
- Configure scan parameters using the web interface
- View real-time progress and results in the browser
- No configuration files needed - all settings in the GUI


## Project Structure
```
port-explorer/
  ├─ src/
  │   ├─ main.rs             # Entry point & CLI mode
  │   ├─ gui.rs              # Web GUI server & API
  │   ├─ config.rs           # Config parsing/validation
  │   ├─ signatures.rs       # Signature loading/matching
  │   ├─ error.rs            # Error types
  │   └─ localisator.rs      # Localization
  ├─ static/                 # Web GUI assets
  │   └─ index.html          # Main GUI interface
  ├─ signatures/             # Service signature YAMLs
  ├─ resources/Localization/ # Localization YAMLs
  ├─ logs/                   # Scan logs
  ├─ config.yaml             # Main config
  ├─ Cargo.toml              # Rust manifest
  └─ README.md               # This ReadMe
```

## Development
- Standard Rust workflow: `cargo build`, `cargo test`, `cargo fmt`
- Add new signatures in `signatures/`
- Add new languages in `resources/Localization/`

## Contributions
- If you encounter an issue/bug, please open an issue.
- If you have feature requests, feel free to either open a pull request or an issue
- New signatures/localization is always welcome

## Issues and Further Development
- Some signatures may be inaccurate and may not work yet
- Testcases need to be expanded to reach 100% coverage and include integration tests
- Currently no more features are planned, but suggestions are welcome.

## License
MIT License

---
Maintainers: SpiceSniper