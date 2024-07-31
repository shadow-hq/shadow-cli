# shadow-cli

[![test-rs](https://github.com/shadow-hq/shadow-cli/actions/workflows/test-rs.yml/badge.svg?branch=main)](https://github.com/shadow-hq/shadow-cli/actions/workflows/test-rs.yml)
![GitHub release (with filter)](https://img.shields.io/github/v/release/shadow-hq/shadow-cli?color=success&label=Latest%20Version)

An open-souce CLI which can be used to clone, compile and upload shadow contracts to the decentralized [Shadow Contract Registry](https://logs.xyz).

## Installation

Ensure that Rust & Cargo are installed:

_Note: Rust >= 1.79.0 is required!_

```bash
curl https://sh.rustup.rs -sSf | sh
```

The CLI's update and installation manager, `shadowup`, can be installed using the following command:

```bash
curl -L https://raw.githubusercontent.com/shadow-hq/shadow-cli/main/shadowup/install | bash
```

If you want to manually install `shadowup`, you can download the latest release from [here](./shadowup/shadowup).

Once you have installed `shadowup`, you can use it to install the Shadow CLI using the following command from a new terminal:

```bash
shadowup
```

After compilation, the `shadow` command will be available to use from a new terminal. For advanced options, see the [shadowup docs](./shadowup)

## Expected Usage

The expected workflow for creating and pushing a new shadow contract group is as follows:

### From Scratch

1. Initialize a new contract group with [`shadow init`](#create-a-contract-group).
2. Add shadow contracts to the contract group with [`shadow fetch`](#create-a-shadow-contract).
3. Modify the shadow contracts as needed.
   1. Optionally, compile the shadow contracts with [`shadow compile`](#compiling-your-shadow-contract).
   2. Optionally, test the shadow contracts with [`shadow sim`](#testing-your-shadow-contract).
4. Push the contract group to the Shadow Contract Registry with [`shadow push`](#uploading-your-contract-group).

### From an Existing Contract Group

1. Clone an existing contract group with [`shadow clone`](#clone-an-existing-contract-group).
2. Modify the shadow contracts as needed.
   1. Optionally, compile the shadow contracts with [`shadow compile`](#compiling-your-shadow-contract).
   2. Optionally, test the shadow contracts with [`shadow sim`](#testing-your-shadow-contract).
3. Push the contract group to the Shadow Contract Registry with [`shadow push`](#uploading-your-contract-group).

---

## Commands

### Create a Contract Group

<details>
<summary>shadow init</summary>
```bash
shadow init
```

This command initializes a new contract group in the current directory. The contract group will have the following structure:

```
ContractGroup_01_01_2000_01_01
├── info.json     # Contains metadata about the contract group
└── README.md     # Contains a README you can fill out with information about the contract group
```

#### Optional Flags

- `--root <path>`: The path to the directory in which to initialize the shadow contract group [default: .]
</details>

### Create a Shadow Contract

<details>
<summary>shadow fetch</summary>

```bash
shadow fetch <contract_address> --etherscan-api-key <etherscan_api_key> --rpc-url <rpc_url>
```

This command fetches a shadow contract and its original compiler settings from Etherscan, and saves it to the current directory.

#### Required Flags

- `<contract_address>`: The address of the shadow contract you wish to fetch
- `--etherscan-api-key <etherscan_api_key>`: Your Etherscan API key. Fetching may not work without this.
- `--rpc-url <rpc_url>`: Your RPC URL. Fetching may not work without this.

#### Optional Flags
- `--root <path>`: The path to the directory in which to save the shadow contract [default: .]
  - *If you wish to save the contract to a contract group, you must either be in the contract group's directory or specify the contract group's directory with the `--root` flag.*
- `--chain <chain>`: The chain on which the shadow contract is deployed
- `--chain-id <chain_id>`: The chain ID on which the shadow contract is deployed
- `--force`: Overwrite the shadow contract if it already exists
</details>

### Clone an Existing Contract Group

<details>
<summary>shadow clone</summary>

```bash
shadow clone <ipfs_cid> --etherscan-api-key <etherscan_api_key> --rpc-url <rpc_url>
```

This command clones an existing contract group from the Shadow Contract Registry and saves it to the current directory.

#### Required Flags
- `<ipfs_cid>`: The IPFS CID of the contract group you wish to clone
- `--etherscan-api-key <etherscan_api_key>`: Your Etherscan API key. Cloning may not work without this.
- `--rpc-url <rpc_url>`: Your RPC URL. Fetching may not work without this.

#### Optional Flags
- `--root <path>`: The path to the directory in which to save the shadow contract [default: .]
  - *If you wish to save the contract to a contract group, you must either be in the contract group's directory or specify the contract group's directory with the `--root` flag.*
- `--chain <chain>`: The chain on which the shadow contract is deployed
- `--chain-id <chain_id>`: The chain ID on which the shadow contract is deployed
- `--force`: Overwrite the shadow contract if it already exists
</details>

### Compiling Your Shadow Contract

<details>
<summary>shadow compile</summary>

```bash
shadow compile --rpc-url <rpc_url>
```

This command compiles the shadow contract in the current directory. The compiled contract will be saved in the `/out` directory, next to the foundry artifact.

_Note: The current working directory MUST contain the shadow contract you wish to compile. This is different from the other commands, which require the `--root` flag to specify the contract group directory._

#### Required Flags
- `--rpc-url <rpc_url>`: Your RPC URL. Compiling may not work without this.

#### Optional Flags
- `--root <path>`: The path to the directory containing the shadow contract [default: .]
</details>

### Testing Your Shadow Contract

<details>
<summary>shadow sim</summary>

```bash
shadow sim <transaction_hash> --rpc-url <rpc_url>
```

This command takes in a transaction hash and simulates the transaction using the contracts in your contract group. The simulation will output logs after the transaction has been executed.

#### Required Flags
- `<transaction_hash>`: The transaction hash of the transaction you wish to simulate
- `--rpc-url <rpc_url>`: Your RPC URL. Simulating may not work without this.

#### Optional Flags
- `--root <path>`: The path to the directory containing the shadow contract group [default: .]
</details>

### Uploading Your Contract Group

<details>
<summary>shadow push</summary>

```bash
shadow push --rpc-url <rpc_url> --pinata-api-key <pinata_api_key> --pinata-secret-api-key <pinata_secret_api_key>
```

This command uploads the contract group in the current directory to the Shadow Contract Registry. The contract group will be pinned to IPFS using Pinata. You will also be prompted to broadcast an EAS attestation on Base in order to have your group appear on https://logs.xyz.

#### Required Flags
- `--rpc-url <rpc_url>`: Your RPC URL. Pushing may not work without this.
- `--pinata-api-key <pinata_api_key>`: Your Pinata API key. Pushing may not work without this.
- `--pinata-secret-api-key <pinata_secret_api_key>`: Your Pinata secret API key. Pushing may not work without this.

#### Optional Flags
- `--root <path>`: The path to the directory containing the shadow contract group [default: .]
</details>

## Getting Help

- Join the [Telegram](https://t.me/shadow_devs) to get help, or
- Open an issue with the [bug](https://github.com/shadow-hq/shadow-reth/issues/new?assignees=&template=bug.yml)

## Contributing

See our [contributing guidelines](./CONTRIBUTING.md).

## Security

This code has not been audited, and should not be used in any production systems.
