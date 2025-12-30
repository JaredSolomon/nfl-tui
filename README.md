# NFL TUI Scoreboard

A terminal-based (TUI) live scoreboard for the NFL and NCAA College Football.

![NFL TUI Demo](https://placehold.co/600x400/png?text=Preview+Coming+Soon)

## Features

*   **Live Scores**: Real-time updates (poll-based).
*   **Field Visualization**: Visual field tracker with custom team colors.
*   **College Football**: Support for NCAA games via `--ncaa`.
*   **Stats**: Possession indicator 🏈, game clock, and broadcast info.
*   **Responsive**: Adapts to terminal size, hides logos on small screens.

## Installation

### From Source
```bash
cargo install --git https://github.com/YOUR_USERNAME/nfl-tui
```

### Manual Build
```bash
git clone https://github.com/YOUR_USERNAME/nfl-tui
cd nfl-tui
cargo run --release
```

## Usage

```bash
# Run NFL Scoreboard (Default)
nfl-tui

# Run College Football Scoreboard
nfl-tui --ncaa

# Set custom update interval (e.g., 5 seconds)
nfl-tui -i 5
```

## Controls

*   `j` / `Down`: Next Game
*   `k` / `Up`: Previous Game
*   `l`: Toggle Logos
*   `q`: Quit
