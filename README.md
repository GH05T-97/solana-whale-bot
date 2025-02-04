# Solana Raydium Trade Monitor Bot

A Telegram bot that monitors trading activity on Raydium DEX (Decentralized Exchange) on the Solana blockchain. The bot tracks both orderbook trades and AMM (Automated Market Maker) swaps for specified tokens.

## Features

- Monitor specific tokens traded on Raydium
- Set custom volume thresholds for monitoring
- Real-time notifications for significant trades
- Track both DEX trades and AMM swaps
- View current hot trading pairs

## Commands

- `/monitorToken <symbol>` - Add a token to monitor (e.g., "SOL")
- `/monitorTokenVolume <min> <max> <timeframe>` - Set volume thresholds for monitoring
  - `min`: Minimum trade volume in USD
  - `max`: Maximum trade volume in USD
  - `timeframe`: Time window in minutes
- `/start` - Begin monitoring
- `/stop` - Stop monitoring

## Prerequisites

- Rust (latest stable version)
- Solana RPC endpoint
- Telegram Bot Token
- Cargo and its dependencies

## Environment Variables

```env
TELEGRAM_BOT_TOKEN=your_bot_token
TELEGRAM_CHAT_ID=your_chat_id
```

## Installation

1. Clone the repository
```bash
git clone https://github.com/yourusername/solana-raydium-monitor.git
cd solana-raydium-monitor
```

2. Build the project
```bash
cargo build --release
```

3. Run the bot
```bash
cargo run
```

## Configuration

- Default RPC endpoint: `https://api.mainnet-beta.solana.com`
- Default volume thresholds: $5,000 - $10,000
- Default monitoring interval: 30 seconds

## Architecture

- `WhaleBot`: Handles Telegram interface and user commands
- `VolumeTracker`: Manages trade monitoring and volume calculations
- Real-time monitoring through Solana RPC
- Price data from Raydium API

## Dependencies

```toml
[dependencies]
teloxide = "0.12"
tokio = { version = "1.0", features = ["full"] }
solana-client = "1.10"
solana-sdk = "1.10"
serde = { version = "1.0", features = ["derive"] }
log = "0.4"
env_logger = "0.10"
reqwest = { version = "0.11", features = ["json"] }
```

## License

MIT

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

---

Would you like me to add any other sections to the README?
