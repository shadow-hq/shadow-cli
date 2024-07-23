# shadow-cli

A simple open-souce CLI which can be used to clone, compile and upload shadow contracts to the [Shadow Contract Registry](https://logs.xyz).

See our [blog post](https://todo.xyz) for more information.

## Installation

You can install the shadow-cli binary by following these steps:

1. Clone this repository
2. Build and install the shadow-cli binary
3. Run the shadow-cli binary
4. Compile the shadow contract

### Clone the repository

```bash
git clone https://github.com/shadow-hq/shadow-cli
cd shadow-cli
```

### Build and install the shadow-cli binary

```bash
cargo install --locked --path bin/shadow-cli --bin shadow
```

### Create your first shadow contract

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

### Compile the shadow contract

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
