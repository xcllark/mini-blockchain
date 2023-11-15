# Mini-Blockchain

Mini-Blockchain is a straightforward implementation of a blockchain, designed to provide an insightful look into the underlying mechanisms of blockchain technology. This project is particularly suited for educational purposes, allowing users to understand and explore the fundamental aspects of blockchain operations.

## Overview

The mini-blockchain project features two main components: a server and a client. The server listens for incoming transactions, storing them in a mempool. After a specified block time, it processes these transactions, updating the blockchain's state. The client is used to send transactions to the server, demonstrating the interaction between different nodes in a blockchain network.

## Getting Started

These instructions will help you get a copy of the project up and running on your local machine for development and educational purposes.

### Prerequisites

Ensure you have the following installed before starting:
- Rust programming language
- Cargo (Rust's package manager)
- Git

### Installation

Clone the repository and build the project:

```bash
# Clone the repository
git clone https://github.com/yourusername/mini-blockchain.git
```

```bash
# Navigate to the project directory
cd mini-blockchain
```

### Usage
```bash
Usage: cargo run <COMMAND>

Commands:
  server  Runs the server and listens to new transactions
  client  Runs the client and tries to connect to the server and send it transactions
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

##### Server Commands

```bash
Usage: cargo run server [OPTIONS]

Options:
  -s, --spec <SPEC>
          Path to the chainspec, if you want preallocations to your address specify it in the chainspec
  -p, --port <PORT>
          Rpc Port [default: 8545]
  -c, --coinbase <COINBASE>
          Coinbase address [default: 0x0000000000000000000000000000000000000000]
      --database-dump <DATABASE_DUMP>
          Path where to dump the database at the end of execution
  -d, --debug
          Whether chain-bit should output debug info to the terminal
          For example, when debug mode is activated, every block will be printed to the terminal
  -r, --report-frequency <REPORT_FREQUENCY>
          How often do you want info about the progress
          Let's you know how many blocks and transactions have been processed [default: 30]
  -b, --block-time <BLOCK_TIME>
          Block time of the blockchain [default: 10]
  -h, --help
          Print help
```

##### Client Commands
```bash
Usage: cargo run client [OPTIONS]

Options:
  -m, --many  Whether to just send one transaction to the client or many from many different clients
  -h, --help  Print help
```


