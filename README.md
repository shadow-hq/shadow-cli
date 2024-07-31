# shadow-cli

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

## Usage

### Create a shadow contract

Let's start by cloning the WETH shadow contract to a new directory.

```bash
shadow fetch 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --root ./shadow-weth
```

This command will create a new directory in the current working directory and pull the verified source code and other contract metadata into it. You can modify the contract code as you normally would, and then compile it using the `shadow compile` command.

For our example, we'll just add a simple `ShadowTransfer` event immediately after the `Transfer` event in the WETH contract.

```solidity
function transferFrom(address src, address dst, uint wad) public returns (bool) {
    ...

    Transfer(src, dst, wad);
    ShadowTransfer();

    ...
}
```

---

### Compiling your shadow contract

```bash
shadow compile --root ./shadow-weth
```

This command will compile the shadow contract via [forge](https://github.com/foundry-rs/foundry), simulate the original contract deployment, and generate shadow compiler artifacts under `./shadow-weth/out/Contract.sol/WETH9.shadow.json` (right next to the original contract artifacts).

WETH9.shadow.json:
```json
{
  "abi": [
    ...,
    {
      "type": "event",
      "name": "ShadowTransfer",
      "inputs": [],
      "anonymous": false
    },
    ...
  ],
  "methodIdentifiers": { ... },
  "bytecode": "0x60606040..."
}
```

---

### Create a contract group

In order to upload your shadow contract to the decentralized Shadow Contract Registry, you need to create a contract group. A contract group is a collection of shadow contracts that are related to each other in some way.

You can simply do this by running the following command:

```bash
shadow init
```

This command will create a new contract group in the current working directory (or, you can change the `--root` flag to specify a different directory). You can then add shadow contracts to this group by running the following command:

```bash
shadow fetch 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --root ./path-to-your-contract-group
```

---

### Uploading your contract group

When you're satisfied with your changes, you can upload your contract group to the Shadow Contract Registry by running the following command:

```bash
shadow push --root ./path-to-your-contract-group
```

This will:

1. Compile all shadow contracts in the contract group.
2. Uploads the contract group's artifacts to IPFS.
3. Prompt you to send a transaction to EAS on Base, attesting that you are the owner of the contract group. (optional, although your changes will not be reflected in the Shadow Contract Registry until you do this)

Note: You must update the contract group's metadata file (`./path-to-your-contract-group/info.json`) before you'll be able to push your changes. You must:

1. Update the `displayName` field to a human-readable name for your contract group.
2. Update the `creator` field to your Ethereum address. This address must be the same as the one you use to sign the EAS transaction.
3. Update the `README.md` file to include a description of your contract group.

---

### Clone an existing contract group

If you would like to clone an existing contract group from IPFS, you can do so by running the following command:

```bash
shadow clone <IPFS_CID>
```

This will clone the contract group to a new directory in the current working directory with the shadowed contracts and metadata.

## Getting Help

- Join the [Telegram](https://t.me/shadow_devs) to get help, or
- Open an issue with the [bug](https://github.com/shadow-hq/shadow-reth/issues/new?assignees=&template=bug.yml)

## Contributing

See our [contributing guidelines](./CONTRIBUTING.md).

## Security

This code has not been audited, and should not be used in any production systems.
