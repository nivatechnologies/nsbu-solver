"""Coverage geometry, empirical refinements and explicit empty-region status."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.regions import coverage, radial_limit
from reference.tuples import quadruple


class RegionTests(unittest.TestCase):
    def test_first_endpoint_complete_and_initial_partial_annulus(self) -> None:
        with mp.workdps(80):
            half = mp.mpf(1)/2
            radius = mp.mpf(3)/10
            for low,high in ((mp.mpf(0),half),(half,mp.mpf(8))):
                complete = coverage(mp.mpf(1)/256,low,high,radius)
                self.assertEqual(complete.status,'Nonempty')
                self.assertEqual(complete.fraction,1)
                self.assertEqual(complete.refinement_change,0)
                self.assertEqual(complete.classification,'EmpiricalQuadrature')
                self.assertEqual(complete.panels,512)
            initial = coverage(mp.mpf(1)/128,half,mp.mpf(8),radius)
            self.assertGreater(initial.fraction,0)
            self.assertLess(initial.fraction,1)
            finer = coverage(mp.mpf(1)/128,half,mp.mpf(8),radius,512)
            self.assertLess(finer.refinement_change,initial.refinement_change)
            self.assertLess(finer.refinement_change,mp.mpf('1e-9'))
            limit,weight = radial_limit(mp.mpf(0),mp.mpf(1)/128,radius)
            self.assertLess(abs(limit-mp.mpf('5.76')),mp.mpf('1e-78'))
            self.assertEqual(weight,(mp.mpf(1)/128)**(mp.mpf(11)/8))

    def test_empty_region_geometry(self) -> None:
        with mp.workdps(80):
            empty = coverage(mp.mpf(1),mp.mpf(1)/2,mp.mpf(8),mp.mpf(3)/10,2)
            self.assertEqual(empty.status,'RegionEmpty')
            self.assertEqual(empty.fraction,0)
            self.assertEqual(empty.refinement_change,0)

    def test_invalid_inputs(self) -> None:
        for values in (('nan','0','1','0.3'),('1','-1','1','0.3'),
                       ('-1','0','1','0.3'),('1','0','1','-0.3'),('0','0','1','0.3'),('1','1','1','0.3'),('1','0','1','0')):
            with self.assertRaises(ValueError):
                coverage(*quadruple(mp.mpf(value) for value in values))
        for panels in (0,1,3):
            with self.assertRaisesRegex(ValueError,'positive and even'):
                coverage(mp.mpf(1),mp.mpf(0),mp.mpf(1),panels=panels)

    def test_declared_quadrature_nodes_and_refinement_order(self) -> None:
        with mp.workdps(80):
            seen: list[mpf] = []
            def record(eta: mpf, tau: mpf, radius: mpf) -> tuple[mpf,mpf]:
                seen.append(eta)
                return radial_limit(eta,tau,radius)
            with patch('reference.regions.radial_limit',side_effect=record):
                coverage(mp.mpf(1)/128,mp.mpf(1)/2,mp.mpf(8),panels=2)
            self.assertEqual(seen,[mp.mpf(v)/4 for v in (-2,0,2,-2,-1,0,1,2)])
            first = coverage(mp.mpf(1)/128,mp.mpf(1)/2,mp.mpf(8),panels=64)
            second = coverage(mp.mpf(1)/128,mp.mpf(1)/2,mp.mpf(8),panels=128)
            ratio = first.refinement_change/second.refinement_change
            self.assertGreater(ratio,15)
            self.assertLess(ratio,17)

    def test_empty_boundary_and_reversed_region(self) -> None:
        with mp.workdps(80):
            for low,expected in (('0.02','Nonempty'),('0.04','Nonempty'),('0.05','RegionEmpty'),('0.10','RegionEmpty')):
                result = coverage(mp.mpf(1),mp.mpf(low),mp.mpf(1)/2,panels=4)
                self.assertEqual(result.status,expected)
            with self.assertRaisesRegex(ValueError,'Invalid coverage geometry'):
                coverage(mp.mpf(1),mp.mpf(1),mp.mpf(1)/2)
