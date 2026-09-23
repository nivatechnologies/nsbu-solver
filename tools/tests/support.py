"""Isolated public-checkout fixtures shared by bootstrap guard tests."""
from pathlib import Path
import shutil
import tempfile
import unittest

CHECKOUT = Path(__file__).resolve().parents[2]


class CheckoutTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix='nsbu-bootstrap-test-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / 'checkout'
        shutil.copytree(CHECKOUT, self.root, ignore=shutil.ignore_patterns(
            '.git', '.venv', 'work', 'target', 'mutants', '.complexipy_cache',
            'checkpoints', '__pycache__', '.DS_Store'))
        # Instrumented subprocess imports read tool configuration from their cwd.
        shutil.copy2(CHECKOUT / 'pyproject.toml', Path(self.temporary.name) / 'pyproject.toml')
