#!/usr/bin/env python3
"""Replace checkout-specific LLVM filenames with repository-relative paths."""
import json
import sys
from pathlib import Path


def main() -> None:
    """Rewrite the report in place after requiring one repository marker."""
    path = Path(sys.argv[1])
    report = json.loads(path.read_text())
    prefix = str(Path.cwd()) + "/"

    def relative(value):
        if isinstance(value, str):
            return value.replace(prefix, "")
        if isinstance(value, list):
            return [relative(item) for item in value]
        if isinstance(value, dict):
            return {key: relative(item) for key, item in value.items()}
        return value

    report = relative(report)
    path.write_text(json.dumps(report, separators=(",", ":")) + "\n")


if __name__ == "__main__":
    main()
