"""Exact-word packet provenance, strict schemas and public command resource/refusal contracts."""
from collections.abc import Mapping
import gzip
import hashlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from reference.reduction_audit.input import HEADER, identity, parse, preflight, prepare
from tools.json_types import Json, array_value, decode, object_value

ROOT=Path(__file__).resolve().parents[2]


def sources(directory: Path) -> tuple[Path,Path]:
    files: list[Path]=[]
    for method in ('cm','ho'):
        raw=gzip.decompress((ROOT/f'evidence/p09/derived-arithmetic/inputs/derived-n4-{method}.json.gz').read_bytes())
        path=directory/(method+'.json')
        path.write_bytes(raw)
        files.append(path)
    return files[0],files[1]


def command(*args: str) -> tuple[int,Mapping[str,Json]]:
    result=subprocess.run([sys.executable,'-m','reference.export_reduction_input',*args],
                          text=True,capture_output=True,check=False)
    return result.returncode,object_value(decode(result.stdout)) if result.stdout else {}


class ReductionInputTests(unittest.TestCase):
    def test_all_roles_rows_and_signed_words_keep_the_original_artifact_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            left,right=sources(Path(temporary))
            data=prepare(left,right,4)
            value=parse(data)
            self.assertEqual(len(data),376968)
            self.assertEqual((value.n,value.points),(4,512))
            self.assertEqual(value.source_hashes,tuple(hashlib.sha256(p.read_bytes()).hexdigest() for p in (left,right)))
            for role,path in enumerate((left,right)):
                root=object_value(decode(path.read_text()))
                for row in (0,12,26,45):
                    bits=array_value(object_value(array_value(root['fields'])[row])['bits'])
                    for point in (0,1,511):
                        self.assertEqual(value.word(role,row,point),bits[point])
            changed=bytearray(data)
            changed[HEADER:HEADER+8]=(1<<63).to_bytes(8,'little')
            signed=parse(bytes(changed))
            self.assertEqual(signed.word(0,0,0),1<<63)
            self.assertEqual(identity(value)['sha256'],hashlib.sha256(data).hexdigest())
            for indices in ((2,0,0),(0,46,0),(0,0,512),(-1,0,0),(0,-1,0),(0,0,-1)):
                with self.assertRaises(ValueError):value.word(*indices)

    def test_truncation_shape_nonfinite_words_and_bad_floors_are_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            data=prepare(*sources(Path(temporary)),4)
            for invalid in (b'',b'x'+data[1:],data[:-1],data+b'x'):
                with self.assertRaises(ValueError):parse(invalid)
            for offset,value in ((8,3),(16,513),(24,0),(24,0x7ff0000000000000),(HEADER,0x7ff8000000000001)):
                changed=bytearray(data);changed[offset:offset+8]=value.to_bytes(8,'little')
                with self.assertRaises(ValueError):parse(bytes(changed))

    def test_cap_identity_and_prescribed_force_are_checked_before_preparation(self) -> None:
        self.assertEqual(preflight(12)['packet_bytes'],10174600)
        for n in (0,6,16):
            with self.assertRaises(ValueError):preflight(n)
        with patch('reference.reduction_audit.input.bounded_bytes',side_effect=AssertionError('No read allowed')):
            with self.assertRaises(ValueError):prepare(Path('absent'),Path('absent'),4,1)
        with tempfile.TemporaryDirectory() as temporary:
            left,right=sources(Path(temporary))
            with self.assertRaisesRegex(ValueError,'method'):prepare(left,left,4)
            import json
            root=dict(object_value(decode(right.read_text())))
            force=list(array_value(root['pressure_force']))
            first=dict(object_value(force[0]));bits=[list(array_value(v)) for v in array_value(first['bits'])]
            bits[0][0]=0x3ff0000000000000
            first['bits']=bits;force[0]=first;root['pressure_force']=force
            right.write_text(json.dumps(root))
            with self.assertRaisesRegex(ValueError,'identical prescribed'):prepare(left,right,4)

    def test_public_command_preflight_execution_and_existing_output_refusal(self) -> None:
        code,report=command('--n','4','--dry-run')
        self.assertEqual(code,0)
        self.assertEqual(report['accepted_pde_windows'],0)
        self.assertEqual(command('--n','4')[0],2)
        self.assertEqual(command('--n','4','--bad')[0],2)
        self.assertEqual(command('--n','4','--dry-run','--cap-bytes','1')[0],1)
        with tempfile.TemporaryDirectory() as temporary:
            left,right=sources(Path(temporary));output=Path(temporary)/'input.bin'
            args=('--n','4','--left',str(left),'--right',str(right),'--output',str(output))
            code,report=command(*args)
            self.assertEqual((code,report['status']),(0,'reduction-input-prepared'))
            original=output.read_bytes()
            self.assertEqual(parse(original).points,512)
            self.assertEqual(command(*args)[0],1)
            self.assertEqual(output.read_bytes(),original)
