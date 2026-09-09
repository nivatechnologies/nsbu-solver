"""Coverage geometry, empirical refinements and explicit empty-region status."""
import unittest
from mpmath import mp
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
                       ('0','0','1','0.3'),('1','1','1','0.3'),('1','0','1','0')):
            with self.assertRaises(ValueError):
                coverage(*quadruple(mp.mpf(value) for value in values))
        for panels in (0,1,3):
            with self.assertRaisesRegex(ValueError,'positive and even'):
                coverage(mp.mpf(1),mp.mpf(0),mp.mpf(1),panels=panels)
