# rbufrp

Fast BUFR (Binary Universal Form for the Representation of meteorological data) decoder with Python bindings.

This is a Python wrapper for the high-performance Rust-based BUFR decoder, providing an easy-to-use API for decoding meteorological data in BUFR format.

## Features

- **High Performance**: Core decoding engine written in Rust
- **Easy to Use**: Simple Python API with full type hints
- **Comprehensive**: Supports BUFR table versions with master, local tables
- **Cross-Platform**: Works on Linux, macOS, and Windows

## Installation

```bash
pip install rbufrp
```

## Quick Start

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

## BUFR Tables

The library includes BUFR tables (master, local) built into the package. You can also specify a custom tables path:

```python
import rbufrp

# Use custom tables directory
rbufrp.set_tables_path("/path/to/custom/tables")

# Or via environment variable
# export RBUFR_TABLES_PATH=/path/to/custom/tables
```

## Platform Support

Pre-built wheels are available for:

- Linux: x86_64, ARM64 (both glibc and musl)
- macOS: Intel (x86_64) and Apple Silicon (ARM64)
- Windows: x86_64

## Requirements

- Python 3.8 or higher
- NumPy

## Building from Source

If a pre-built wheel is not available for your platform, you can build from source:

```bash
pip install maturin
pip install rbufrp --no-binary rbufrp
```

This requires:
- Rust toolchain 1.70 or higher
- Python development headers

## License

MIT License - see LICENSE file for details.

## Links

- GitHub: https://github.com/yourusername/rbufr
- Documentation: https://github.com/yourusername/rbufr
- Issue Tracker: https://github.com/yourusername/rbufr/issues

## Acknowledgments

- BUFR specifications from WMO (World Meteorological Organization)
- Built with PyO3 and maturin
