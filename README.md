# Crypto Portfolio Manager

![Rust](https://img.shields.io/badge/Rust-1.82-orange?logo=rust)
![License](https://img.shields.io/badge/license-MIT-blue)
![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Contributions Welcome](https://img.shields.io/badge/contributions-welcome-brightgreen)

A production-grade Rust application for managing a $100 micro-cap cryptocurrency portfolio (Phala Network, Sui, Dusk Network) with a focus on maximizing returns by January 30, 2026. 

Built with a "10x engineer" mindset, it features three console-based screens for real-time portfolio monitoring, sentiment analysis, and live market updates, complemented by SMS and email notifications for critical events. 

The application is secure, optimized, and runs locally using Redis for caching and PostgreSQL for persistent storage, with an exchange-agnostic design supporting multiple APIs (e.g., CoinGecko, LunarCrush).

---

## 🚀 Features

- **Three Console Screens**:
  - **Portfolio Status**: Displays holdings, cash, total value, and sentiment in a Bloomberg/Jane Street-style table using `comfy-table`.
  - **Sentiment Analysis Dashboard**: Shows real-time sentiment scores, data sources, cache TTL, and trade recommendations (Hold/Buy, Monitor, Sell).
  - **Live Market Updates**: Tracks all cryptocurrencies, with PHA, SUI, DUSK pinned to the top, sorted by market cap or 24h price change.
- **Notifications**:
  - SMS (Twilio) and email (SendGrid) alerts for:
    - Significant actions (stop-loss sales, rebalancing).
    - Major portfolio value changes (>10%) or holding price changes (>15%).
    - Sentiment shifts (>0.2) or breaking news (e.g., LunarCrush galactic score spikes).
- **Exchange-Agnostic**: Modular `Exchange` and `SentimentProvider` traits support multiple APIs, configurable via `config.toml`.
- **Security**:
  - API keys stored in `.env`.
  - HMAC-SHA256 signed logs in production.
  - HTTPS API calls and input validation.
- **Optimization**:
  - Redis caching for prices and sentiment (configurable TTL).
  - Async `tokio` tasks for non-blocking I/O.
  - Lightweight dependencies for low CPU/memory usage.
- **Local Execution**: Runs entirely on your machine with PostgreSQL and Redis, no cloud/VPS dependencies.
- **Dev/Prod Support**: Configurable intervals, logging levels, and notification settings for development and production environments.

---

## 🏛️ Architecture

The application follows a modular, layered architecture to ensure maintainability, scalability, and performance. Below is a high-level overview:

```mermaid
graph TD
    A[Main.rs] --> B[Portfolio Screen]
    A --> C[Sentiment Dashboard]
    A --> D[Market Updates]
    B --> E[Portfolio.rs]
    C --> F[SentimentProvider]
    D --> G[MarketProvider]
    E --> H[Exchange.rs]
    F --> H
    G --> H
    E --> I[Database.rs]
    F --> I
    G --> I
    E --> J[Notification.rs]
    F --> J
    I --> K[Redis]
    I --> L[PostgreSQL]
    J --> M[Twilio API]
    J --> N[SendGrid API]
    H --> O[CoinGecko API]
    F --> P[LunarCrush API]
    A --> Q[Config.rs]
    A --> R[Logger.rs]
```

### Key Components
- **Main.rs**: Orchestrates three console screens via separate processes (`portfolio`, `sentiment`, `market`).
- **Portfolio.rs**: Manages holdings, stop-loss (20%), rebalancing (60% max allocation), and value tracking.
- **Exchange.rs**: Fetches prices via a trait-based, exchange-agnostic system.
- **SentimentProvider**: Retrieves sentiment scores (0–1) for PHA, SUI, DUSK.
- **MarketProvider**: Fetches live market data, pinning portfolio coins.
- **Database.rs**: Handles Redis caching and PostgreSQL trade logging.
- **Notification.rs**: Sends SMS/email alerts for significant events.
- **Config.rs**: Parses `config.toml` for environment-specific settings.
- **Logger.rs**: Logs actions with HMAC-SHA256 signatures in production.


## 📋 Prerequisites
 - **Rust**: Version 1.82 or later (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **PostgreSQL**: Local installation (`sudo apt install postgresql` on Linux, `brew install postgresql on macOS`)
- **Redis**: Local installation (`sudo apt install redis` on Linux, `brew install redis on macOS`)
- **Terminal Emulator** (Linux/macOS): `xterm` (`sudo apt install xterm` or `brew install xterm`)
- **API Accounts**:
    - CoinGecko (optional, free tier available)
    - LunarCrush (for sentiment analysis)
    - Twilio (for SMS notifications)
    - SendGrid (for email notifications)

## 🛠️ Installation
- **Clone the Repository**:
    ```
    git clone https://github.com/sdg7onado/crypto-portfolio-manager.git

    cd crypto-portfolio-manager
    ```

- **Install Dependencies**:

    ```
    cargo build
    ```

- **Set Up Databases**:
    - Start PostgreSQL:
        ```
        sudo service postgresql start  # Linux

        brew services start postgresql  # macOS

        createdb portfolio
        ```

    - **Start Redis**:
        ```
        redis-server
        ```

- **Configure Environment**:
    - Create `.env` in the project root (do not commit):

        ```
        SENTIMENT_API_KEY=your_lunarcrush_key
        TWILIO_ACCOUNT_SID=your_twilio_sid
        TWILIO_AUTH_TOKEN=your_twilio_token
        TWILIO_PHONE_NUMBER=+12345678901
        RECIPIENT_PHONE_NUMBER=+19876543210
        SENDGRID_API_KEY=your_sendgrid_key
        RECIPIENT_EMAIL=your_email@example.com
        POSTGRES_USER=user
        POSTGRES_PASSWORD=password
        POSTGRES_DB=portfolio
        REDIS_URL=redis://localhost:6379
        ```

- **Configure Application**:
    - Update `config.toml`:
    
        ```
        environment = "dev"

        [[exchanges]]
        name = "coingecko"
        api_key = ""
        api_secret = ""
        base_url = "https://api.coingecko.com/api/v3"

        [database]
        postgres_url = "postgresql://user:password@localhost:5432/portfolio"

        [redis]
        url = "redis://localhost:6379"

        [portfolio]
        check_interval_secs = 86400
        max_allocation = 0.6
        stop_loss_percentage = 0.2

        [sentiment]
        api_url = "https://api.lunarcrush.com/v2"
        api_key = "${SENTIMENT_API_KEY}"
        cache_ttl_secs = 3600
        positive_threshold = 0.7
        negative_threshold = 0.3

        [display]
        sentiment_refresh_secs = 86400
        use_colors = true

        [market]
        refresh_secs = 60
        sort_by = "market_cap"
        pinned_symbols = ["phala-network", "sui", "dusk-network"]

        [notification]
        sms_enabled = true
        email_enabled = true
        twilio_account_sid = "${TWILIO_ACCOUNT_SID}"
        twilio_auth_token = "${TWILIO_AUTH_TOKEN}"
        twilio_phone_number = "${TWILIO_PHONE_NUMBER}"
        recipient_phone_number = "${RECIPIENT_PHONE_NUMBER}"
        sendgrid_api_key = "${SENDGRID_API_KEY}"
        sender_email = "portfolio@example.com"
        recipient_email = "${RECIPIENT_EMAIL}"
        [notification.notification_thresholds]
        portfolio_value_change_percent = 10.0
        holding_value_change_percent = 15.0
        sentiment_change = 0.2
        ```

 ## 🚀 Usage
 - **Run in Development Mode**:
    ```
    RUST_LOG=debug cargo run
    ```

    - Spawns three console windows: portfolio, sentiment, and market.
    - Frequent updates (`market.refresh_secs = 60`).
    - Verbose logging for debugging.

 - **Run in Production Mode**:
    ```
    RUST_LOG=info cargo build --release
    ./target/release/crypto-portfolio-manager
    ```

    - Optimized performance with signed logs.
    - Longer update intervals (`market.refresh_secs = 300`).

 - **Provide Daily Updates**:
    - Submit price and sentiment data for PHA, SUI, DUSK (e.g., via CoinGecko, LunarCrush).
    - Example:
        - PHA: $0.22, sentiment 0.75
        - SUI: $3.10, sentiment 0.60
        - DUSK: $0.24, sentiment 0.25

    - Market data: Top 100 coins by market cap or 24h change.

- **Monitor Notifications**:
    - SMS: Short alerts (75–115 characters) for significant actions, portfolio changes, or sentiment shifts.
    - Email: Detailed HTML reports with timestamps and metrics.

 ## 📊 Example Output
 ### Portfolio Status Screen
```
=== Portfolio Status ===
+----------------+----------+----------------+------------+---------------+-----------+
| Symbol         | Quantity | Purchase Price | Stop-Loss  | Current Value | Sentiment |
+----------------+----------+----------------+------------+---------------+-----------+
| phala-network  | 250.00   | $0.20          | $0.16      | $50.00        | 0.75      |
| sui            | 10.00    | $3.00          | $2.40      | $30.00        | 0.60      |
| dusk-network   | 80.00    | $0.25          | $0.20      | $20.00        | 0.25      |
| Cash           | $0.00    |                |            |               |           |
| Total          |          |                |            | $100.00       |           |
+----------------+----------+----------------+------------+---------------+-----------+
```

### Sentiment Analysis Dashboard
```
=== Sentiment Analysis Dashboard ===
Timestamp: 2025-07-31T13:36:00Z
+----------------+--------------------+--------------+-----------+-----------------+
| Symbol         | Sentiment Score    | Data Source  | Cache TTL | Recommendation  |
+----------------+--------------------+--------------+-----------+-----------------+
| phala-network  | 0.75               | Redis Cache  | 3500s     | Hold/Buy        |
| sui            | 0.60               | API Fetch    | N/A       | Monitor         |
| dusk-network   | 0.25               | Redis Cache  | 3200s     | Sell            |
+----------------+--------------------+--------------+-----------+-----------------+
```

### Live Market Updates
```
=== Live Market Updates ===
Timestamp: 2025-07-31T13:36:00Z
+----------------+--------------+-----------------+------------------+
| Symbol         | Price (USD)  | Market Cap (USD)| 24h Change (%)   |
+----------------+--------------+-----------------+------------------+
| phala-network  | $0.22        | $150000000      | +10.00%          |
| sui            | $3.10        | $250000000      | +3.33%           |
| dusk-network   | $0.24        | $100000000      | -4.00%           |
| bitcoin        | $118050.85   | $2300000000000  | +2.50%           |
| ethereum       | $3500.00     | $420000000000   | -1.20%           |
| solana         | $180.00      | $80000000000    | +5.00%           |
| ...            | ...          | ...             | ...              |
+----------------+--------------+-----------------+------------------+
```

### 🔔 Notifications
- **SMS (Stop-Loss)**:
```
Portfolio Action: dusk-network: Negative sentiment triggered at $0.24 (sentiment: 0.25), sold 80 tokens for $19.20. 2025-07-31T13:36:00Z
```

- **Email (Portfolio Value Change)**:
```
<h2>Portfolio Value Change Alert</h2>
<p><strong>Change:</strong> 12.50%</p>
<p><strong>Previous:</strong> $100.00</p>
<p><strong>Current:</strong> $112.50</p>
<p><strong>Timestamp:</strong> 2025-07-31T13:36:00Z</p>
```
 ## 🛡️ Security
 - **API Keys**: Stored in `.env`, never committed to version control.
- **Logging**: HMAC-SHA256 signed logs in production (`portfolio_log.txt`) for integrity.
- **Network**: HTTPS for all API calls (CoinGecko, LunarCrush, Twilio, SendGrid).
- **Input Validation**: Robust error handling for API responses and database queries.
- **Notification Throttling**: Cached in Redis to prevent duplicate alerts within 1 hour.

## ⚡ Performance
- **Caching**: Redis stores prices and sentiment (5-min and 1-hour TTL, respectively).
- **Async Operations**: tokio for non-blocking I/O in API calls and database queries.
- **Lightweight**: Optimized with `opt-level = 3`, `lto = "thin"`, and minimal dependencies.
- **Local Execution**: Runs on localhost, minimizing latency and resource usage.

## 📈 Workflow
```mermaid

sequenceDiagram
    participant User
    participant App as Crypto Portfolio Manager
    participant Redis
    participant PostgreSQL
    participant APIs as CoinGecko/LunarCrush/Twilio/SendGrid

    User->>App: Run (cargo run)
    App->>Redis: Initialize cache
    App->>PostgreSQL: Initialize trade log
    App->>App: Spawn 3 console windows
    loop Every check_interval_secs
        App->>APIs: Fetch prices (PHA, SUI, DUSK)
        App->>Redis: Cache prices
        App->>APIs: Fetch sentiments
        App->>Redis: Cache sentiments
        App->>PostgreSQL: Log trades
        App->>App: Check stop-loss/rebalancing
        alt Significant action
            App->>APIs: Send SMS/email
            App->>PostgreSQL: Log action
        end
        alt Major change (value >10%, sentiment >0.2)
            App->>APIs: Send SMS/email
        end
        App->>User: Update portfolio screen
        App->>User: Update sentiment dashboard
        App->>APIs: Fetch market data
        App->>User: Update market screen
    end
```

## 🤝 Contributing
Contributions are welcome! Please follow these steps:

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/your-feature`).
3. Commit changes (`git commit -m "Add your feature"`).
4. Push to the branch (`git push origin feature/your-feature`).
5. Open a pull request.

## 📜 License
 This project is licensed under the MIT License. See the LICENSE file for details. 
 
 ## 📬 Contact
 For questions or feedback, open an issue on GitHub or contact the maintainer at okechukwu.agufuobi@hotmail.com (mailto:okechukwu.agufuobi@hotmail.com).
 
 ## 🎯 Roadmap
 - Add support for additional exchanges (Binance, Kraken).
- Implement advanced sentiment analysis with local NLP (e.g., TextBlob).
- Support WebSocket APIs for real-time market updates.
- Add graphical dashboard option using `ratatui`.


