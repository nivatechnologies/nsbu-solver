"""Complete bit inventory, unprojected force binding and failed-aggregate negative controls."""
import copy
import gzip
from pathlib import Path
import unittest
from mpmath import mp
from reference.cyclic.bits import FixedForcing
from reference.cyclic.results import reality_defect
from reference.cyclic.schema import decode_fixture
from reference.derived.comparison import preflight
from reference.derived.reduction import Reduction
from reference.derived.schema import bind_endpoint_force, derived
from tools.json_types import JsonObject, array_value, object_value

ROOT=Path(__file__).resolve().parents[2]


def fixture() -> JsonObject:
    return dict(object_value(decode_fixture(gzip.decompress((ROOT/'fixtures/reference/derived-n4-cm.json.gz').read_bytes()))))


class DerivedContracts(unittest.TestCase):
    def test_full_real_export_and_exact_unprojected_endpoint_input_bind(self) -> None:
        with mp.workdps(120):
            actual=derived(fixture(),4,'CM')
            force=FixedForcing(gzip.decompress((ROOT/'evidence/p09/arithmetic/inputs/force-n4.json.gz').read_bytes()),4,16*1024**2)
            bind_endpoint_force(actual.force,force)
            self.assertEqual(len(actual.fields),46)
            self.assertEqual(len(actual.force),343)
            self.assertEqual(len(actual.state),27)
            self.assertLess(reality_defect(actual.state),mp.mpf('1e-15'))
            for wave in ((3,0,0),(-3,0,0)):
                actual.force[wave]=(mp.mpc(1),mp.mpc(0),mp.mpc(0))
            with self.assertRaisesRegex(ValueError,'Diagnostic pressure force'):
                bind_endpoint_force(actual.force,force)

    def test_missing_reordered_wrong_kind_and_nonfinite_fields_are_refused(self) -> None:
        original=fixture()
        for key,value in (('samples',12),('schema','changed'),('diagnostic_scalar_transforms',54),('rhs_calls',True)):
            root=copy.deepcopy(original);root[key]=value
            with self.assertRaises(ValueError):
                derived(root,4,'CM')
        root=copy.deepcopy(original);root['fields']=list(array_value(root['fields']))[:-1]
        with self.assertRaises(ValueError):
            derived(root,4,'CM')
        for change in ('kind','size','infinity','boolean','orders'):
            root=copy.deepcopy(original);rows=list(array_value(root['fields']))
            row=dict(object_value(rows[0]));bits=list(array_value(row['bits']))
            if change=='kind':row['quantity']='pressure'
            elif change=='size':bits.pop()
            elif change=='infinity':bits[0]=0x7ff0000000000000
            elif change=='boolean':bits[0]=True
            else:row['orders']=[0,0,1]
            row['bits']=bits;rows[0]=row;root['fields']=rows
            with self.assertRaises(ValueError):
                derived(root,4,'CM')

    def test_opposite_vectors_and_ordered_tensor_entries_use_full_differences(self) -> None:
        total=Reduction.new(2)
        for a,b in ((1,-1),(0,0),(0,0)):
            total.add([mp.mpc(a)]*2,[mp.mpc(b)]*2)
        result=total.finish(3,mp.mpf(1))
        self.assertEqual(result['rms_error'],'2.0')
        self.assertEqual(result['sampled_peak_error'],'2.0')
        tensor=Reduction.new(2)
        for _ in range(9):tensor.add([mp.mpc(1)]*2,[mp.mpc(0)]*2)
        self.assertEqual(tensor.finish(9,mp.mpf(1))['rms_error'],'3.0')

    def test_failed_partial_reductions_and_preflight_cannot_produce_evidence(self) -> None:
        with self.assertRaises(ValueError):Reduction.new(0)
        total=Reduction.new(2)
        with self.assertRaises(ValueError):total.add([mp.mpc(1),mp.mpc(mp.nan)],[mp.mpc(0)]*2)
        with self.assertRaises(ValueError):total.finish(0,mp.mpf(1))
        with self.assertRaises(ValueError):total.add([mp.mpc(0)]*2,[mp.mpc(0)]*2)
        total=Reduction.new(2)
        with self.assertRaises(ValueError):total.add([mp.mpc(0)],[mp.mpc(0)]*2)
        total=Reduction.new(2)
        for expected,floor in ((1,mp.mpf(1)),(0,mp.mpf(0)),(0,mp.inf)):
            with self.assertRaises(ValueError):total.finish(expected,floor)
        with self.assertRaises(ValueError):preflight(12,1)
        self.assertEqual(preflight(12,32*1024**3)['profiles'],6)
