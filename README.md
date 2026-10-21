# entropick : CLI Password Generator

![CI](https://github.com/sabarish-vm/entropick/actions/workflows/tests.yml/badge.svg)

A no non-sense offline CLI password generator with custom pattern syntax.
For now, the output is print to the CLI along with displaying the entropy estimates from `zxcvbn` library from dropbox.

```
entropick generate --pattern '[[a-z]][[A-Z]][[0-9]][[!@#$%^&*]]' --length 16
```

## Features

- **Custom pattern syntax** — define password rules using `[[mandatory]]` and `((optional))` blocks
- **CSPRNG** — randomness from `getrandom` (OS kernel) via ChaCha20 with Fisher-Yates shuffling of characters
- **Profile management** — save named patterns for reuse via TOML config file
- **Cross-platform** — Linux, macOS, Windows

## Installation

```bash
git clone https://github.com/sabarish-vm/entropick
cd entropick
cargo build --release
cp target/release/entropick ~/.local/bin/
```

## Pattern Syntax

The core of entropick is its bracket-based pattern language:

| Syntax      | Meaning                      |
| ----------- | ---------------------------- |
| `[[a-z]]`   | Atleast one lowercase letter |
| `[[A-Z]]`   | Atleast one uppercase letter |
| `[[0-9]]`   | Atleast one digit            |
| `[[!@#$%]]` | Atleast one literal symbol   |
| `((!@#$%))` | Optional symbol pool         |

**Rules:**

- Each `[[…]]` block expands to a flat `Vec<char>` (e.g. `a-z` → `[a, b, …, z]`)
- Ranges must be `a-z`, `A-Z`, `0-9` with start ≤ end. Additional examples : `a-c`, `d-k`, `4-8`
- Pattern syntax is not regex-like, for e.g. multiple dashes in one block (`a-zA-Z`) is an error — use separate blocks
- At least one `[[…]]` block is required
- Characters outside brackets are silently ignored

### Examples

```
# Minimum viable
entropick generate --pattern '[[a-z]][[A-Z]][[0-9]]' --length 8

# With optional symbols
entropick generate --pattern '[[a-z]][[A-Z]][[0-9]]((!@#$%^&*))' --length 16

# Digits only, longer
entropick generate --pattern '[[0-9]]' --length 32
```

## Usage

### Generate passwords

```bash
# From pattern + length
entropick generate --pattern '[[a-z]][[A-Z]][[0-9]]' --length 16

# From a saved profile
entropick profile github --pattern '[[a-z]][[A-Z]][[0-9]]' --length 16

# From a saved profile
entropick generate --profile github

# Override the profile's stored length
entropick generate --profile github --length 32

# Generate multiple at once
entropick generate --pattern '[[0-9]]' --length 8 --count 5
```

### Manage profiles

```bash
# Add a profile
entropick profile add github --pattern '[[a-z]][[A-Z]s[[0-9]]((!@#$%))' --length 24

# List all profiles
entropick profile list

# Show a profile's details
entropick profile show github

# Edit specific fields
entropick profile edit github --length 32

# Remove a profile
entropick profile remove github

# Search profiles by name
entropick profile search git
```

## Development

```bash
# Run all tests (tests need single-threaded execution)
cargo test -- --test-threads=1

# Build release
cargo build --release

# Run the binary
cargo run -- generate --pattern '[[a-z]][[A-Z]][[0-9]]' --length 16
```
