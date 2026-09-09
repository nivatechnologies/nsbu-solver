"""Integrity guards must reject changes in either lexical direction."""
import hashlib
import json
from pathlib import Path
from typing import Protocol, cast
from unittest.mock import patch
from tools.check_repository import FROZEN_SHA256, MANIFEST_SHA256, PROBLEM_SHA256, check_case, check_metadata, check_repository, check_review, sha256, unfenced_lines
from tools.json_types import Json, array_value, decode, object_value
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

    def test_metadata_requires_exact_names_and_boolean_false(self) -> None:
        path = self.root/'project-status.json'
        original = dict(object_value(decode(path.read_text())))
        replacements: tuple[tuple[str,Json],...] = (('project','AAA'),('project','ZZZ'),
            ('license','0BSD'),('license','ZZZ'),('niva_dependency',0),
            ('niva_dependency',None),('niva_dependency','false'))
        for key,value in replacements:
            changed = dict(original)
            changed[key] = value
            path.write_text(json.dumps(changed))
            with self.assertRaises(ValueError):
                check_metadata(self.root)

    def test_hash_guards_in_both_directions(self) -> None:
        for invalid in ('0'*64,'f'*64):
            with patch('tools.check_repository.sha256',return_value=invalid):
                with self.assertRaisesRegex(ValueError,'Original review manifest'):
                    check_review(self.root)
            with patch.dict(FROZEN_SHA256,{'COMPLETE_DESIGN.md':invalid}):
                with self.assertRaisesRegex(ValueError,'Frozen baseline'):
                    check_review(self.root)

    def test_component_names_cardinality_and_packet_metadata(self) -> None:
        path = self.root/'docs/design/navier-runtime-review-manifest.json'
        original = dict(object_value(decode(path.read_text())))
        entries = [dict(object_value(v)) for v in array_value(original['components'])]
        swapped = [dict(v) for v in entries]
        swapped[0]['file'] = 'different-component.md'
        def admit_manifest(candidate: Path) -> str:
            return MANIFEST_SHA256 if candidate == path else sha256(candidate)
        malformed: dict[str,Json] = {}
        for components in ([*entries,entries[0]],swapped,[malformed]):
            changed = dict(original)
            changed['components'] = components
            path.write_text(json.dumps(changed))
            with patch('tools.check_repository.sha256',side_effect=admit_manifest):
                with self.assertRaisesRegex(ValueError,'exactly the 12'):
                    check_review(self.root)
        path.write_text(json.dumps(original))
        with patch('tools.check_repository.sha256',side_effect=admit_manifest):
            result = check_review(self.root)
        self.assertEqual(result['components_verified'],12)
        self.assertIs(result['packet_verified'],True)
        self.assertIs(check_case(self.root)['benchmark_copy_identical'],True)

    def test_different_self_consistent_problems_are_refused(self) -> None:
        public = self.root/'benchmarks/similarity-mms-v2.json'
        frozen = self.root/'docs/design/similarity-mms-v2.json'
        original = dict(object_value(decode(public.read_text())))
        found: set[bool] = set()
        # Fixed changed problems bracket the canonical identity lexically.
        for number in (0,1):
            changed = dict(original)
            problem = dict(object_value(original['mathematical_problem']))
            problem['different_problem'] = number
            canonical = json.dumps(problem,sort_keys=True,separators=(',',':'),ensure_ascii=True).encode('ascii')
            digest = hashlib.sha256(canonical).hexdigest()
            found.add(digest < PROBLEM_SHA256)
            changed['mathematical_problem'] = problem
            changed['problem_identity'] = {'sha256':digest}
            for path in (public,frozen):
                path.write_text(json.dumps(changed))
            with self.assertRaisesRegex(ValueError,'Mathematical problem identity mismatch'):
                check_case(self.root)
        self.assertEqual(found,{False,True})

    def test_nested_packet_name_is_refused_before_reading_identical_bytes(self) -> None:
        path = self.root/'docs/design/navier-runtime-review-manifest.json'
        manifest = dict(object_value(decode(path.read_text())))
        packet = dict(object_value(manifest['packet']))
        name = str(packet['file'])
        nested = self.root/'docs/design/zz'/name
        nested.parent.mkdir()
        nested.write_bytes((self.root/'docs/design'/name).read_bytes())
        packet['file'] = 'zz/'+name
        manifest['packet'] = packet
        path.write_text(json.dumps(manifest))
        def admit_manifest(candidate: Path) -> str:
            return MANIFEST_SHA256 if candidate == path else sha256(candidate)
        with patch('tools.check_repository.sha256',side_effect=admit_manifest):
            with self.assertRaisesRegex(ValueError,'nonlocal component name'):
                check_review(self.root)
