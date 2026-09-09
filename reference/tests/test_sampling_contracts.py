"""Sampling uses signed cell coordinates and the declared arithmetic/resource profile."""
from itertools import product
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.evaluator import Evaluation
from reference.verify_sampling import difference, run, sample_force
from tools.json_types import JsonObject


class SamplingContractTests(unittest.TestCase):
    def test_signed_coordinates_and_nonunit_conditioning(self) -> None:
        with mp.workdps(80):
            seen: list[tuple[mpf,mpf,mpf]] = []
            zero = mp.mpf(0)
            vector = (mp.mpf(7),zero,zero)
            def field(x: mpf, y: mpf, z: mpf, time: mpf) -> Evaluation:
                self.assertEqual(time,mp.mpf(1)/256)
                seen.append((x,y,z))
                return Evaluation(vector,zero,vector,zero,(zero,),
                                  ((mp.mpf(1),zero,zero,zero),(zero,zero,zero,zero),(zero,zero,zero,zero)),0,((zero,zero,zero),)*3)
            with patch('reference.verify_sampling.evaluate',side_effect=field):
                _,report = sample_force(8,mp.mpf(1)/256,80,4*1024**3)
            expected = set(product((mp.mpf(v)/8 for v in (-4,-3,-2,-1,0,1,2,3)),repeat=3))
            self.assertEqual(set(seen),expected)
            self.assertEqual(len(seen),512)
            self.assertEqual(report['max_assembly_conditioning'],'0.14285714285714285714')
            with self.assertRaisesRegex(ValueError,'sampling requires'):
                sample_force(4,mp.mpf(1)/64,80,4*1024**3)

    def test_distinct_grid_comparisons_and_declared_profiles(self) -> None:
        calls: list[tuple[int,str,int,int,int]] = []
        def sampled(n: int, time: mpf, precision: int, cap: int) -> tuple[Spectrum,JsonObject]:
            calls.append((n,str(time),precision,cap,mp.dps))
            return {(0,0,0):(mp.mpc(n),mp.mpc(0),mp.mpc(0))},{}
        with patch('reference.verify_sampling.sample_force',side_effect=sampled):
            run()
            for error in ('1e-60','2e-60'):
                with patch('reference.verify_sampling.difference',return_value={'max_component_change':error}):
                    with self.assertRaisesRegex(ArithmeticError,'arithmetic refinement'):
                        run()
        expected = [(n,str(mp.mpf(time)),p,4*1024**3,p)
                    for time in ('1/1024','1/256') for n,p in ((4,80),(8,80),(12,80),(12,120))]
        self.assertEqual(calls[:8],expected)

    def test_noninteger_full_band_norm_rendering(self) -> None:
        with mp.workdps(80):
            coarse: Spectrum = {(0,0,0):(mp.mpc(1),mp.mpc(0),mp.mpc(0))}
            fine: Spectrum = {(0,0,0):(mp.mpc(mp.mpf(4)/3),mp.mpc(0),mp.mpc(0))}
            self.assertEqual(difference(coarse,fine),{
                'full_fine_band_l2':'0.33333333333333333333',
                'full_fine_band_h1':'0.33333333333333333333',
                'max_component_change':'0.33333333333333333333'})
