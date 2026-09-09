"""Adversarial cases for each independent repository-validation boundary."""
from pathlib import Path
from unittest.mock import patch
from tools.check_repository import (FROZEN_SHA256, MANIFEST_SHA256, check_case,
                              check_links, check_metadata, check_public_content,
                              check_review, public_files, unfenced_lines)
from tools.json_types import Json, array_value, object_value, string_value
from tools.tests.support import CheckoutTestCase


class RepositoryChecks(CheckoutTestCase):
    def rewrite_manifest(self, text: str) -> None:
        (self.root / 'docs/design/navier-runtime-review-manifest.json').write_text(text)

    def test_manifest_schema_and_traversal(self) -> None:
        with patch('tools.check_repository.sha256',return_value=MANIFEST_SHA256):
            self.rewrite_manifest('{"components":[]}')
            with self.assertRaisesRegex(ValueError,'exactly the 12'):
                check_review(self.root)
        original = (self.root / 'docs/design/navier-runtime-review-manifest.json')
        source = Path(__file__).resolve().parents[2] / 'docs/design/navier-runtime-review-manifest.json'
        original.write_bytes(source.read_bytes())
        text = original.read_text().replace('navier-runtime-review-packet.md','../outside.md')
        self.rewrite_manifest(text)
        # Admit only the altered manifest here, then exercise its separate path guard.
        import tools.check_repository as check_repository
        actual_hash = check_repository.sha256
        def hash_manifest(path: Path) -> str:
            return MANIFEST_SHA256 if path == original else actual_hash(path)
        with patch('tools.check_repository.sha256',side_effect=hash_manifest):
            with self.assertRaisesRegex(ValueError,'nonlocal'):
                check_review(self.root)

    def test_independent_frozen_hash_guard(self) -> None:
        with patch.dict(FROZEN_SHA256,{'COMPLETE_DESIGN.md':'0'*64}):
            with self.assertRaisesRegex(ValueError,'Frozen baseline'):
                check_review(self.root)

    def test_case_identity_and_false_evidence(self) -> None:
        name = 'similarity-mms-v2.json'
        original = (self.root / 'benchmarks' / name).read_text()
        changed = original.replace('ba81b770','00000000')
        for folder in ('benchmarks','docs/design'):
            (self.root / folder / name).write_text(changed)
        with self.assertRaisesRegex(ValueError,'identity mismatch'):
            check_case(self.root)
        for flag in ('source_instance','verified_pde_convergence'):
            for folder in ('benchmarks','docs/design'):
                (self.root / folder / name).write_text(original.replace(f'"{flag}": false',f'"{flag}": true'))
            with self.assertRaisesRegex(ValueError,'must not claim'):
                check_case(self.root)

    def test_symlink_and_private_text_refused(self) -> None:
        link = self.root / 'outside'
        link.symlink_to(self.root / 'LICENSE')
        with self.assertRaisesRegex(ValueError,'symlink'):
            public_files(self.root)
        link.unlink()
        text = self.root / 'private.md'
        text.write_text('/'+'home/'+'fixture/'+'private')
        with self.assertRaisesRegex(ValueError,'Personal absolute path'):
            check_public_content(self.root)

    def test_link_escape_and_fence_delimiters(self) -> None:
        with self.assertRaisesRegex(ValueError,'escapes checkout'):
            check_links(self.root,self.root/'README.md','[bad](../../escape.md)')
        self.assertEqual(unfenced_lines('before\n````\n```\ninside\n````\nafter'),'before\nafter')
        with self.assertRaisesRegex(ValueError,'Unclosed'):
            unfenced_lines('```python\nmissing close')
        self.assertEqual(check_links(self.root,self.root/'README.md','[web](https://example.com)\n[anchor](#x)'),0)

    def test_metadata_and_license_sections(self) -> None:
        path = self.root/'project-status.json'
        original = path.read_text()
        for old,new in (('NSBU Solver','Other'),('Apache-2.0','MIT'),('"niva_dependency": false','"niva_dependency": true')):
            path.write_text(original.replace(old,new))
            with self.assertRaises(ValueError):
                check_metadata(self.root)
        path.write_text(original)
        (self.root/'LICENSE').write_text('Apache License')
        with self.assertRaisesRegex(ValueError,'missing an expected'):
            check_metadata(self.root)

    def test_json_shapes(self) -> None:
        self.assertEqual(array_value([1,2]),[1,2])
        self.assertEqual(string_value('text'),'text')
        invalid_objects: tuple[Json,...] = (None,42,'not an object',[])
        for value in invalid_objects:
            with self.assertRaises(ValueError):
                object_value(value)
        invalid_arrays: tuple[Json,...] = (None,42,'not an array',{})
        for value in invalid_arrays:
            with self.assertRaises(ValueError):
                array_value(value)
        with self.assertRaises(ValueError):
            string_value(42)
