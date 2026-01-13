"""
Type stubs for rbufrp._core

This file provides type hints for the Rust extension module.
"""

from typing import List, Optional, Iterator, Any

class BUFRDecoder:
    """BUFR decoder for parsing BUFR files."""
    
    def __init__(self) -> None:
        """Create a new BUFR decoder instance."""
        ...
    
    def decode(self, bytes: bytes) -> BUFRFile:
        """
        Decode BUFR data from raw bytes.

        Args:
            bytes: Raw BUFR data as bytes

        Returns:
            BUFRFile: Parsed BUFR file containing messages

        Raises:
            IOError: If the data cannot be read
            ValueError: If the data is not valid BUFR format
        """
        ...
    
    def parse_message(self, message: BUFRMessage) -> BUFRParsed:
        """
        Parse a single BUFR message.
        
        Args:
            message: The BUFR message to parse
            
        Returns:
            BUFRParsed: Parsed data from the message
            
        Raises:
            Exception: If parsing fails
        """
        ...

class BUFRFile:
    """
    Represents a parsed BUFR file containing one or more messages.
    This class is iterable and supports len().
    """

    def __repr__(self) -> str:
        """Return a string representation of the BUFR file."""
        ...

    def __len__(self) -> int:
        """
        Get the number of messages in the file.

        Returns:
            int: Number of BUFR messages
        """
        ...

    def __iter__(self) -> Iterator[BUFRMessage]:
        """
        Return an iterator over the BUFR messages.

        Returns:
            Iterator[BUFRMessage]: Iterator over all messages
        """
        ...

    def __next__(self) -> BUFRMessage:
        """
        Return the next BUFR message in iteration.

        Returns:
            BUFRMessage: Next message

        Raises:
            StopIteration: When no more messages are available
        """
        ...

    def message_count(self) -> int:
        """
        Get the number of messages in the file.

        Returns:
            int: Number of BUFR messages
        """
        ...

    def get_message(self, index: int) -> BUFRMessage:
        """
        Get a specific message by index.

        Args:
            index: Zero-based index of the message

        Returns:
            BUFRMessage: The requested message

        Raises:
            IndexError: If the index is out of range
        """
        ...

class BUFRMessage:
    """
    Represents a single BUFR message.
    """
    
    def __repr__(self) -> str:
        """Return a string representation of the message."""
        ...
    
    def version(self) -> int:
        """
        Get the BUFR edition/version number.
        
        Returns:
            int: BUFR edition (typically 2, 3, or 4)
        """
        ...

    def section2(self) -> Optional[Section2]:
        """
        Get Section 2 of the BUFR message, if present.

        Returns:
            Optional[Section2]: Section 2 object or None if not present
        """


class BUFRParsed:
    """
    Represents parsed BUFR data.

    This class contains the decoded meteorological data from a BUFR message.
    This class is iterable and indexable.
    """

    iter_index: int

    def __repr__(self) -> str:
        """
        Return a formatted string representation of the parsed data.

        Returns:
            str: Human-readable representation of all records
        """
        ...

    def __iter__(self) -> Iterator[BUFRRecord]:
        """
        Return an iterator over the BUFR records.

        Returns:
            Iterator[BUFRRecord]: Iterator over all records
        """
        ...

    def __next__(self) -> BUFRRecord:
        """
        Return the next BUFR record in iteration.

        Returns:
            BUFRRecord: Next record

        Raises:
            StopIteration: When no more records are available
        """
        ...

    def __len__(self) -> int:
        """
        Get the number of records in the parsed data.

        Returns:
            int: Number of records
        """
        ...

    def __getitem__(self, index: int) -> BUFRRecord:
        """
        Get a record by index. Supports negative indexing.

        Args:
            index: Index of the record (can be negative)

        Returns:
            BUFRRecord: The requested record

        Raises:
            IndexError: If the index is out of range
        """
        ...

    def record_count(self) -> int:
        """
        Get the number of records in the parsed data.

        Returns:
            int: Number of records
        """
        ...

    def get_record(self, key: str) -> List[BUFRRecord]:
        """
        Get all records matching the specified key/name.

        Args:
            key: The name/key to search for

        Returns:
            List[BUFRRecord]: List of matching records (may be empty)
        """
        ...

class BUFRRecord:
    """
    Represents a single BUFR data record.

    A record contains a key (name) and a value, which can be:
    - A single value (number, string, or None for missing)
    - A list of values (for repeated data)
    - A NumPy array (for array data)
    """

    def __repr__(self) -> str:
        """
        Return a string representation of the record.

        Returns:
            str: String representation
        """
        ...

    def key(self) -> Optional[str]:
        """
        Get the key (name) of this record.

        Returns:
            Optional[str]: The record name, or None if unnamed
        """
        ...

    def value(self) -> Any:
        """
        Get the value of this record.

        The return type depends on the data:
        - float: For single numeric values
        - str: For single string values
        - None: For missing values
        - List[Union[float, str, None]]: For repeated values
        - numpy.ndarray: For array data

        Returns:
            Any: The record value in an appropriate Python type
        """
        ...

class Section2:
    """
    Represents Section 2 of a BUFR message.

    Section 2 contains optional metadata about the BUFR message.
    """

    def __repr__(self) -> str:
        """
        Return a string representation of Section 2.

        Returns:
            str: String representation
        """
        ...

    def len(self) -> int:
        """
        Get the length of Section 2 in bytes.

        Returns:
            int: Length in bytes
        """
        ...

    def is_empty(self) -> bool:
        """
        Check if Section 2 is empty.

        Returns:
            bool: True if Section 2 is empty, False otherwise
        """
        ...

    def get_raw_bytes(self) -> bytes:
        """
        Get the raw bytes of Section 2.

        Returns:
            bytes: Raw byte content of Section 2
        """
        ...

    

def set_tables_path(path: str) -> None:
    """
    Set the base path for BUFR table files.
    
    This function configures where the decoder should look for BUFR table files
    (Table B, Table D, etc.) needed for decoding messages.
    
    Args:
        path: Absolute path to the directory containing BUFR tables
        
    Example:
        >>> import rbufrp
        >>> rbufrp.set_tables_path("/usr/share/bufr/tables")
    """
    ...

def get_tables_path() -> str:
    """
    Get the currently configured base path for BUFR table files.
    
    Returns:
        str: Current tables directory path
        
    Example:
        >>> import rbufrp
        >>> print(rbufrp.get_tables_path())
        /usr/share/bufr/tables
    """
    ...

__all__ = [
    "BUFRDecoder",
    "BUFRFile",
    "BUFRMessage",
    "BUFRParsed",
    "BUFRRecord",
    "Section2",
    "set_tables_path",
    "get_tables_path",
]
