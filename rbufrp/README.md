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
from rbufrp import BUFRDecoder

# Read BUFR file
with open("data.bufr", "rb") as f:
    bufr_data = f.read()

# Create decoder and decode file
decoder = BUFRDecoder()
bufr_file = decoder.decode(bufr_data)

# Iterate through messages
for message in bufr_file:
    print(message)
    print("BUFR Version:", message.version())

    # Parse each message to get records
    parsed = decoder.parse_message(message)

    # Iterate through records
    for record in parsed:
        print(record)
        # Access record data
        if record.key():
            print(f"{record.key()}: {record.value()}")
```

## Advanced Usage

### Accessing Specific Messages

```python
# Get specific message by index
bufr_file = decoder.decode(bufr_data)
message = bufr_file.get_message(0)

# Get message count
print(f"Total messages: {bufr_file.message_count()}")
print(f"Total messages: {len(bufr_file)}")
```

### Working with Parsed Records

```python
parsed = decoder.parse_message(message)

# Get record count
print(f"Total records: {parsed.record_count()}")
print(f"Total records: {len(parsed)}")

# Access records by index (supports negative indexing)
first_record = parsed[0]
last_record = parsed[-1]

# Search for records by key
temperature_records = parsed.get_record("AIR TEMPERATURE")
for record in temperature_records:
    print(f"{record.key()}: {record.value()}")
```

### Accessing Section 2 (Optional Metadata)

```python
for message in bufr_file:
    section2 = message.section2()
    if section2 is not None:
        print(f"Section 2 length: {section2.len()}")
        print(f"Section 2 is empty: {section2.is_empty()}")
        raw_bytes = section2.get_raw_bytes()
```

## BUFR Tables

The library includes BUFR tables (master, local) built into the package. You can also specify a custom tables path:

```python
import rbufrp

# Use custom tables directory
rbufrp.set_tables_path("/path/to/custom/tables")

# Check current tables path
print(rbufrp.get_tables_path())

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
