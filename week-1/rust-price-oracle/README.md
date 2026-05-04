# Simple SOL Price Guessing Game

A terminal-based guessing game where you try to predict the hidden SOL price.

## Features
- Guess the price between $50 – $300 using `rand`
- Colored terminal UI via `colored`
- 5 tries to win

## Project Structure

```text
src/
├── main.rs   # Entry point
├── ui.rs     # Welcome banner
├── game.rs   # Game loop
└── hints.rs  # Hint logic
```

## Run
cargo run

## Dependencies
- colored = "3"
- rand = "0.10"
