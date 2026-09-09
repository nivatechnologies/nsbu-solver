"""Small work probes run before expensive reference studies in mutation checks."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.jets import Jet
from reference.regions import coverage
from reference.steps import cm_weights, ho_tableau


class BoundedReferenceWorkTests(unittest.TestCase):
    def test_coefficient_context_budget(self) -> None:
        with mp.workdps(80):
            original = mp.workdps
            def bounded(precision: int) -> object:
                self.assertEqual(precision,100)
                return original(precision)
            with patch.object(mp,'workdps',side_effect=bounded):
                self.assertEqual(cm_weights(mp.mpf(0))[0],mp.mpf(1)/2)
                self.assertEqual(ho_tableau(mp.mpf(0))[0][1][0],mp.mpf(1)/2)

    def test_constant_jet_has_zero_derivative(self) -> None:
        with mp.workdps(80):
            for axis in range(4):
                derivative = Jet.constant(0).derivative(axis)
                self.assertEqual(derivative,Jet.constant(0))

    def test_region_refinement_doubles_the_requested_work(self) -> None:
        panels_seen: list[int] = []
        def integrated(_tau: mpf, _low: mpf, _high: mpf, _radius: mpf, panels: int) -> mpf:
            self.assertIn(panels,(16,32))
            panels_seen.append(panels)
            return mp.mpf(1)
        with patch('reference.regions.integral',side_effect=integrated):
            result = coverage(mp.mpf(1)/128,mp.mpf(0),mp.mpf(1),panels=16)
        self.assertEqual(panels_seen,[16,32])
        self.assertEqual(result.panels,32)
