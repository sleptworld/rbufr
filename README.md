# rbufr

Fast BUFR (Binary Universal Form for the Representation of meteorological data) decoder written in Rust with Python bindings.

## Features

- **Fast**: Written in Rust for high-performance decoding
- **Python Bindings**: Easy-to-use Python API via PyO3
- **Cross-Platform**: Supports Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), and Windows
- **Comprehensive**: Supports BUFR table versions with master, local, and OPERA tables
- **Type Safe**: Full type hints for Python API

## Installation

### From GitHub Release

Download the appropriate wheel for your platform from the [releases page](https://github.com/yourusername/rbufr/releases):

```bash
pip install rbufrp-0.1.0-cp38-abi3-linux_x86_64.whl
```

### From Source

Requires Rust toolchain (1.70+) and Python 3.8+:

```bash
cd rbufrp
pip install maturin
maturin develop --release
```

## Usage

### Python

```python
import rbufrp

# Decode a BUFR file
with open("data.bufr", "rb") as f:
    bufr_data = f.read()

decoder = rbufrp.BUFRDecoder()
parsed = decoder.decode(bufr_data)

# Access decoded records
for record in parsed.records:
    print(f"{record.name}: {record.values} {record.unit}")
```

### Rust

```rust
use rbufr::decoder::BUFRDecoder;

fn main() {
    let data = std::fs::read("data.bufr").unwrap();
    let decoder = BUFRDecoder::new();
    let parsed = decoder.decode(&data).unwrap();

    for record in parsed.records {
        println!("{:?}: {:?} {:?}", record.name, record.values, record.unit);
    }
}
```

## BUFR Tables

The library includes BUFR tables (master, local, OPERA) in the `rbufr/tables` directory. The Python package automatically locates these tables, or you can specify a custom path:

```python
import rbufrp

# Use custom tables directory
rbufrp.set_tables_path("/path/to/custom/tables")

# Or via environment variable
# export RBUFR_TABLES_PATH=/path/to/custom/tables
```

## Architecture

- **rbufr**: Core Rust library for BUFR decoding
- **rbufrp**: Python bindings using PyO3
- **Tables**: BUFR table definitions for different data formats

## Development

### Prerequisites

- Rust 1.70+
- Python 3.8+
- maturin

### Building

```bash
# Build Rust library
cd rbufr
cargo build --release

# Build Python package
cd ../rbufrp
maturin build --release
```

### Testing

```bash
# Run Rust tests
cd rbufr
cargo test

# Run Python tests
cd ../rbufrp
pytest
```

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux | x86_64 (glibc) | ✅ |
| Linux | x86_64 (musl) | ✅ |
| Linux | ARM64 (glibc) | ✅ |
| Linux | ARM64 (musl) | ✅ |
| macOS | x86_64 (Intel) | ✅ |
| macOS | ARM64 (Apple Silicon) | ✅ |
| Windows | x86_64 | ✅ |

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- BUFR specifications from WMO (World Meteorological Organization)
- Built with [PyO3](https://pyo3.rs) and [maturin](https://www.maturin.rs)
