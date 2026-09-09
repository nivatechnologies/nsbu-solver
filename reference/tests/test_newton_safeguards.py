"""Fault injection checks proposal safeguards separately from the true residual."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.evaluator import implicit_root
from reference.jets import Jet
from reference.scalar import rational, root


class NewtonSafeguardTests(unittest.TestCase):
    def test_out_of_bracket_and_stagnating_proposals_use_bisection(self) -> None:
        with mp.workdps(80):
            z,time = mp.mpf(1)/8,mp.mpf(1)/512
            expected = root(z,time)
            for exponent in (-3,-100):
                def degraded_jacobian(text: str) -> mpf:
                    # Only the Newton derivative is perturbed. The equation,
                    # admissible bracket and acceptance residual stay exact.
                    return mp.mpf(exponent) if text == '-3/4' else rational(text)
                with patch('reference.scalar.rational',side_effect=degraded_jacobian):
                    result = root(z,time,512)
                self.assertLessEqual(result.iterations,512)
                self.assertLess(abs(result.value-expected.value),mp.mpf('1e-75'))
                self.assertLessEqual(result.lower,result.value)
                self.assertLessEqual(result.value,result.upper)
                residual = result.value-z*z*result.value**(mp.mpf(1)/4)-(mp.mpf(1)/128-time)
                self.assertLessEqual(abs(residual),16*mp.eps*(mp.mpf(1)/128-time))

    def test_reviewed_three_sweep_implicit_jet_budget(self) -> None:
        with mp.workdps(80):
            divisions: list[int] = []
            original = Jet.__truediv__
            def counted(numerator: Jet, denominator: Jet | mpf | int) -> Jet:
                if isinstance(denominator,Jet):
                    divisions.append(1)
                return original(numerator,denominator)
            with patch.object(Jet,'__truediv__',new=counted):
                result = implicit_root(Jet.variable(mp.mpf(1)/8,2),Jet.variable(mp.mpf(1)/512,3))
            self.assertEqual(len(divisions),3)
            self.assertLess(max(abs(v) for v in result.residual.coefficients),mp.mpf('1e-70'))

    def test_fixed_coordinate_work_fixtures(self) -> None:
        with mp.workdps(80):
            for z_text,t_text,cap in (('16352/100000','19158/12800000',7),
                                      ('23276/100000','7202/12800000',7),
                                      ('36708/100000','51834/12800000',8),
                                      ('40542/100000','83852/12800000',9)):
                result = root(mp.mpf(z_text),mp.mpf(t_text),cap)
                self.assertLessEqual(result.iterations,cap)
                self.assertGreater(result.value,0)
                self.assertLessEqual(abs(result.residual),16*mp.eps*(mp.mpf(1)/128-mp.mpf(t_text)))
