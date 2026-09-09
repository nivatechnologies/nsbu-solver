"""Work-budget probes fail before accidental enormous integer powers or PDE work."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.pilot import pilot, reservation
from reference.steps import RightHandSide


class BoundedDivisor(int):
    def __rpow__(self, base: int, modulo: int | None = None) -> int:
        raise AssertionError(f'The timestep divisor must not become an integer exponent (base={base}, modulo={modulo})')


class PilotWorkTests(unittest.TestCase):
    def test_class_reservations_are_added_without_exponential_work(self) -> None:
        with patch('reference.pilot.preflight',return_value={'reserved_bytes':2}):
            _,cache,total = reservation(4,8192,80,'CM',1024**3)
        self.assertEqual(cache,21565440)
        self.assertEqual(total,21565442)

    def test_divisor_is_a_bounded_scale_not_an_exponent(self) -> None:
        divisor = BoundedDivisor(8192)
        with self.assertRaisesRegex(AssertionError,'integer exponent'):
            divisor.__rpow__(20)
        calls = 0
        def step(state: Spectrum, time: mpf, dt: mpf, _rhs: RightHandSide, _nu: mpf) -> Spectrum:
            nonlocal calls
            self.assertLess(calls,32)
            self.assertEqual(time,mp.mpf(calls)/8192)
            self.assertEqual(dt,mp.mpf(1)/8192)
            calls += 1
            return state
        with patch('reference.pilot.cm_step',side_effect=step),\
             patch('reference.pilot.ho_step',side_effect=AssertionError('wrong integration method')),\
             patch('reference.pilot.tracking',return_value={}):
            result = pilot(4,divisor,80,'CM',1024**3)
        self.assertEqual(calls,32)
        self.assertEqual(result['steps_completed'],32)
