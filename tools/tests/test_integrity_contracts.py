"""Integrity guards must reject changes in either lexical direction."""
import json
from pathlib import Path
from typing import Protocol, cast
from unittest.mock import patch
from tools.check_repository import MANIFEST_SHA256, check_case, check_metadata, check_repository, check_review, sha256, unfenced_lines
from tools.json_types import Json, decode, object_value
from tools.tests.support import CheckoutTestCase


class MaterializedAlias(Protocol):
    __value__: object


class IntegrityContracts(CheckoutTestCase):
    def test_public_report_contract(self) -> None:
        report = check_repository(self.root)
        self.assertEqual(report['schema_version'],1)
        self.assertEqual(report['status'],'passed')
        self.assertEqual(report['scope'],'bootstrap packaging only')
        self.assertEqual(report['errors'],[])
        self.assertEqual(set(report['checks']),{'review','case','metadata','public_content'})
        self.assertIs(check_metadata(self.root)['private_dependency_declared'],False)

    def test_identity_both_directions(self) -> None:
        public = self.root/'benchmarks/similarity-mms-v2.json'
        frozen = self.root/'docs/design/similarity-mms-v2.json'
        original = public.read_bytes()
        for changed in (b' '+original,b'~'+original):
            public.write_bytes(changed)
            with self.assertRaisesRegex(ValueError,'copy differs'):
                check_case(self.root)
        for prefix in ('00000000','ffffffff'):
            changed = original.replace(b'ba81b770',prefix.encode('ascii'))
            public.write_bytes(changed)
            frozen.write_bytes(changed)
            with self.assertRaisesRegex(ValueError,'identity mismatch'):
                check_case(self.root)

    def test_component_size_and_digest_are_separate_guards(self) -> None:
        path = self.root/'docs/design/navier-runtime-review-manifest.json'
        original = path.read_text()
        def admit_manifest(candidate: Path) -> str:
            return MANIFEST_SHA256 if candidate == path else sha256(candidate)
        manifest = object_value(decode(original))
        packet = dict(object_value(manifest['packet']))
        for byte_delta in (-1,1):
            changed = dict(manifest)
            new_packet = dict(packet)
            new_packet['bytes'] = int(str(packet['bytes']))+byte_delta
            changed['packet'] = new_packet
            path.write_text(json.dumps(changed))
            with patch('tools.check_repository.sha256',side_effect=admit_manifest):
                with self.assertRaisesRegex(ValueError,'Preserved component'):
                    check_review(self.root)
        for expected_hash in ('0'*64,'f'*64):
            changed = dict(manifest)
            new_packet = dict(packet)
            new_packet['sha256'] = expected_hash
            changed['packet'] = new_packet
            path.write_text(json.dumps(changed))
            with patch('tools.check_repository.sha256',side_effect=admit_manifest):
                with self.assertRaisesRegex(ValueError,'Preserved component'):
                    check_review(self.root)

    def test_longer_closing_fence_and_lazy_json_alias(self) -> None:
        self.assertEqual(unfenced_lines('before\n```text\nhidden\n`````\nafter'),'before\nafter')
        # PEP 695 evaluates aliases lazily. Materialize the declared JSON union so
        # malformed aliases cannot evade runtime tests merely by remaining unused.
        representation = str(cast(MaterializedAlias,Json).__value__)
        for member in ('None','bool','int','float','str','Sequence','Mapping'):
            self.assertIn(member,representation)
