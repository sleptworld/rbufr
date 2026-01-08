"""
Type stubs for rbufrp._core

This file provides type hints for the Rust extension module.
"""

class BUFRDecoder:
    """BUFR decoder for parsing BUFR files."""
    
    def __init__(self) -> None:
        """Create a new BUFR decoder instance."""
        ...
    
    def decode(self, file_path: str) -> BUFRFile:
        """
        Decode a BUFR file from the given path.
        
        Args:
            file_path: Path to the BUFR file to decode
            
        Returns:
            BUFRFile: Parsed BUFR file containing messages
            
        Raises:
            IOError: If the file cannot be read
            ValueError: If the file is not a valid BUFR file
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
    """
    
    def __repr__(self) -> str:
        """Return a string representation of the BUFR file."""
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

class BUFRParsed:
    """
    Represents parsed BUFR data.
    
    This class contains the decoded meteorological data from a BUFR message.
    """
    
    def __repr__(self) -> str:
        """
        Return a formatted string representation of the parsed data.
        
        Returns:
            str: Human-readable representation of all records
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
    "set_tables_path",
    "get_tables_path",
]
