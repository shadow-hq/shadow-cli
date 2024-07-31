# `shadowup`

Bifrost is the shadow cli's installer and version manager.

## Installation
```bash
curl -L https://raw.githubusercontent.com/shadow-hq/shadow-cli/main/shadowup/install | bash
```

## Usage

To install the latest stable release:
```bash
shadowup
```

To install the latest stable release (pre-compiled):
```bash
shadowup --binary
```

To install a specific branch:
```bash
shadowup --version <branch>
```

To install a specific tag:
```bash
shadowup --version <tag>
```

To install the latest nightly commit:
```bash
shadowup +nightly
```

To install a specific tag (pre-compiled):
```bash
shadowup --version <tag> --binary
```

To list all available versions:
```bash
shadowup --list
```

To update bifrost to the latest version:
```bash
shadowup --update
```
