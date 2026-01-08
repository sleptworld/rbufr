"""
rbufrp - BUFR (Binary Universal Form for the Representation of meteorological data) decoder
"""

import os
from pathlib import Path
from typing import Optional, Union

# Import the Rust extension module
from ._core import (
    set_tables_path,
    get_tables_path,
    BUFRDecoder,
    BUFRFile,
    BUFRMessage,
    BUFRParsed,
)

__version__ = "0.1.0"
__all__ = [
    "BUFRDecoder",
    "BUFRFile",
    "BUFRMessage",
    "BUFRParsed",
    "set_tables_path",
    "get_tables_path",
    "initialize_tables_path",
]


def _find_tables_directory() -> Optional[Path]:
    env_path = os.environ.get("RBUFR_TABLES_PATH")
    if env_path:
        tables_path = Path(env_path)
        if tables_path.exists() and tables_path.is_dir():
            return tables_path

    package_dir = Path(__file__).parent
    installed_tables = package_dir / "tables"
    if installed_tables.exists() and installed_tables.is_dir():
        return installed_tables

    dev_tables = package_dir.parent.parent.parent / "rbufr" / "tables"
    if dev_tables.exists() and dev_tables.is_dir():
        return dev_tables

    cwd_tables = Path.cwd() / "tables"
    if cwd_tables.exists() and cwd_tables.is_dir():
        return cwd_tables

    return None


def initialize_tables_path(custom_path: Optional[Union[str, Path]] = None) -> None:
    if custom_path:
        custom_path = Path(custom_path)
        if not custom_path.exists():
            raise RuntimeError(f"指定的 tables 路径不存在: {custom_path}")
        set_tables_path(str(custom_path.absolute()))
        return

    tables_dir = _find_tables_directory()
    if tables_dir is None:
        raise RuntimeError(
            "无法找到 BUFR tables 目录。请执行以下操作之一:\n"
            "1. 设置环境变量 RBUFR_TABLES_PATH\n"
            "2. 使用 initialize_tables_path('/path/to/tables') 手动指定\n"
            "3. 确保在包含 tables 目录的位置运行"
        )

    set_tables_path(str(tables_dir.absolute()))


# 自动初始化 tables 路径
try:
    initialize_tables_path()
except RuntimeError as e:
    import warnings
    warnings.warn(
        f"Tables 路径自动初始化失败: {e}\n"
        "您可以稍后手动调用 rbufrp.initialize_tables_path() 来设置",
        UserWarning
    )


def main() -> None:
    """命令行入口点"""
    print(f"Tables path: {get_tables_path()}")


if __name__ == "__main__":
    main()
