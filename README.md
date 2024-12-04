# Escrow Solana Project

## Overview

This project implements a Solana-based smart contract for token escrow, enabling secure token transfers between a depositor and a recipient. The escrow ensures funds can only be withdrawn by the recipient under specific conditions, such as an expiry time.

## Features

- Supports SPL Token 2022 standards.
- Automated testing suite for core functionalities.
- Deployed on Solana devnet for ease of access and testing.

## Deployed Program Information

- **Program ID:** FQsrCdTzAVkqg6eTximoptrxpMERQ5A2uZ6VjcBnGWo9
- **Explorer Link:** [View on Solana Explorer](https://explorer.solana.com/address/FQsrCdTzAVkqg6eTximoptrxpMERQ5A2uZ6VjcBnGWo9?cluster=devnet)

---

## Installation and Setup

### Prerequisites

Ensure you have the following:

- Node.js version 14 or higher
- Yarn or npm package manager
- Solana CLI installed and configured
- Anchor framework installed and configured

Clone the repository and install dependencies. Ensure the Anchor CLI is properly set up for Solana program development.

### Clone the Repository

```bash
git clone https://github.com/School-of-Solana/program-SchneiderNejc.git
cd escrow_solana
```

### Install Dependencies

```bash
yarn install
# or
npm install
```

### Configure Anchor

```bash
cargo install --git https://github.com/coral-xyz/anchor --tag v0.30.1 anchor-cli --locked
```

---

## Usage

### Test the Program

Run the test suite to verify program functionality. The tests cover:

1. Creating an escrow.
2. Funding the escrow with tokens.
3. Withdrawing tokens from the escrow after the expiry condition.
   Run the following command to execute the tests:

```bash
anchor test
```

### Deploy the Program

The program is pre-deployed on the devnet. You can redeploy it if required by building the program and deploying it using Anchor.

```bash
anchor build
anchor deploy --provider.cluster devnet
```

---

## Key Functions

### Creating an Escrow

- **Purpose:** Initializes an escrow with a specified token amount and expiry time.
- **Accounts Involved:**
  - Escrow PDA
  - Depositor's token account
  - Recipient's wallet
  - Mint address

### Funding the Escrow

- **Purpose:** Transfers the specified tokens from the depositor to the escrow account.
- **Accounts Involved:**
  - Depositor's token account
  - Escrow PDA's token account

### Withdrawing from Escrow

- **Purpose:** Allows the recipient to withdraw tokens from the escrow once the expiry conditions are met.
- **Accounts Involved:**
  - Escrow PDA's token account
  - Recipient's token account

---

## Development Notes

### Helper Functions

- Airdrop SOL to fund accounts for testing.
- Confirm transactions to ensure successful completion on the blockchain.

---

## Additional Resources

- Solana Devnet: A testing environment for Solana programs.
- Anchor Framework Documentation: Comprehensive guide for Solana smart contract development.
- Solana SPL Token 2022 Guide: Details on the updated token standard.

---

## License

This project is licensed under the MIT License. Refer to the LICENSE file for details.

---

### Contact

For questions or contributions, feel free to reach out at [nejc.sch@gmail.com].
