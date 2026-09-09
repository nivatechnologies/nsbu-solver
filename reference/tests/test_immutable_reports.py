"""Immutable reference results cannot be rewritten after verification."""
from dataclasses import FrozenInstanceError
import unittest
from mpmath import mp
from reference.evaluator import evaluate, implicit_root
from reference.jets import Jet
from reference.regions import coverage
from reference.scalar import CoordinateUnresolved, root
from reference.verify_fields import FieldSample, scalar_force
from reference.verify_steps import Study


class ImmutableReportTests(unittest.TestCase):
    def test_result_objects_reject_field_assignment(self) -> None:
        with mp.workdps(80):
            zero = mp.mpf(0)
            evaluated = evaluate(zero,zero,zero,zero)
            scalar = root(zero,zero)
            solution = implicit_root(Jet.variable(zero,2),Jet.variable(zero,3))
            region = coverage(mp.mpf(1)/256,zero,mp.mpf(1)/2,panels=2)
            sample = FieldSample('origin',('0','0','0','0'),evaluated,zero)
            study = Study(80,'CM',{}, {}, {},zero)
            objects: tuple[tuple[object,str],...] = ((evaluated,'divergence'),(scalar,'residual'),
                (solution,'jet'),(region,'fraction'),(sample,'label'),(study,'precision'))
            for value,attribute in objects:
                with self.assertRaises(FrozenInstanceError):
                    setattr(value,attribute,None)

    def test_actual_scalar_force_uses_valid_startup_time_domain(self) -> None:
        with mp.workdps(80):
            point = (mp.mpf(1)/32,mp.mpf(1)/64,mp.mpf(1)/10,mp.mpf(0))
            self.assertLess(max(abs(value) for value in scalar_force(point)),mp.mpf('1e-75'))

    def test_unresolvable_coordinate_exhausts_with_typed_failure(self) -> None:
        with mp.workdps(80):
            with self.assertRaisesRegex(CoordinateUnresolved,'iteration cap exhausted'):
                root(mp.mpf('1e100'),mp.mpf(0),64)
