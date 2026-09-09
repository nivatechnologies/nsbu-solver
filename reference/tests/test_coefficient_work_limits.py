"""Declared precision provisioning and bounded coefficient/root work."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.scalar import CoordinateUnresolved, rational, root
from reference.steps import coefficient_guard_digits, ho_phi, ho_tableau


class CoefficientWorkTests(unittest.TestCase):
    def test_guard_digit_provisioning_and_extreme_endpoint(self) -> None:
        with mp.workdps(80):
            for argument,expected in (('0',20),('-1',20),('-10',21),('-10000',24),('-1e309',329)):
                self.assertEqual(coefficient_guard_digits(mp.mpf(argument)),expected)

    def test_recurrence_does_not_consume_a_taylor_budget(self) -> None:
        with mp.workdps(80):
            z = -mp.mpf(3)/2
            for order in (1,2,3):
                expected = mp.hyp1f1(1,order+1,z)/mp.factorial(order)
                self.assertLess(abs(ho_phi(z,order,1)-expected),mp.mpf('1e-75'))
            with self.assertRaises(ArithmeticError):
                ho_phi(mp.mpf(-1),1,1)

    def test_default_series_capacity_boundary(self) -> None:
        # At z=-1, term magnitudes are reciprocal factorials. These precisions
        # bracket the default 512-term capacity in the pinned mpmath profile.
        with mp.workdps(1165):
            value = ho_phi(mp.mpf(-1),1)
            self.assertLess(abs(value-(1-mp.exp(-1))),mp.mpf('1e-1164'))
        with mp.workdps(1166):
            with self.assertRaisesRegex(ArithmeticError,'sum exhausted'):
                ho_phi(mp.mpf(-1),1)

    def test_root_work_cap_at_residual_acceptance_boundary(self) -> None:
        # Fixed rational points inside the periodic cell exercise a final
        # residual between 15 and 16 eps*tau and a nearby rejected iteration.
        with mp.workdps(80):
            for z_text,t_text,cap in (('47261/100000','71258/12800000',8),
                                      ('48393/100000','31867/12800000',9)):
                time = mp.mpf(t_text)
                result = root(mp.mpf(z_text),time,cap)
                self.assertLessEqual(result.iterations,cap)
                self.assertLessEqual(abs(result.residual),16*mp.eps*(mp.mpf(1)/128-time))

    def test_root_reports_actual_work_and_honors_default_cap(self) -> None:
        with mp.workdps(80):
            evaluations: list[str] = []
            def counted(text: str) -> mpf:
                if text == '1/4':
                    evaluations.append(text)
                return rational(text)
            with patch('reference.scalar.rational',side_effect=counted):
                result = root(mp.mpf(1)/8,mp.mpf(1)/512)
                self.assertEqual(result.iterations,len(evaluations))
                evaluations.clear()
                with self.assertRaises(CoordinateUnresolved):
                    root(mp.mpf('1e100'),mp.mpf(0))
                self.assertEqual(len(evaluations),2048)

    def test_axis_root_is_exact_without_iterations(self) -> None:
        with mp.workdps(80):
            for time in (mp.mpf(0),mp.mpf(1)/256,mp.mpf(127)/16384):
                result = root(mp.mpf(0),time,1)
                tau = mp.mpf(1)/128-time
                self.assertEqual((result.value,result.lower,result.upper),(tau,tau,tau))
                self.assertEqual(result.residual,0)
                self.assertEqual(result.iterations,0)
            with self.assertRaisesRegex(ValueError,'Require finite'):
                root(mp.mpf(0),mp.mpf(1)/64)

    def test_ho_scaled_extreme_negative_coefficients(self) -> None:
        with mp.workdps(80):
            z = -mp.mpf('1e308')
            rows,weights = ho_tableau(z)
            for actual,expected in ((z*z*weights[0],-1),(z*z*weights[4],4),(z*weights[3],-1)):
                self.assertLess(abs(actual-expected),mp.mpf('1e-70'))
            limits = ((),(-1,),(1,-2),(1,-1,-1),(mp.mpf(-1)/4,)*4)
            for row,expected_row in zip(rows,limits):
                for value,expected in zip(row,expected_row):
                    self.assertLess(abs(z*value-expected),mp.mpf('1e-70'))

    def test_taylor_work_cap_requires_relative_term_accuracy(self) -> None:
        with mp.workdps(80):
            for order,numerator,cap in ((1,31,46),(1,56,52),(2,7,35),(3,9,36)):
                z = -mp.mpf(numerator)/100
                with self.assertRaisesRegex(ArithmeticError,'sum exhausted'):
                    ho_phi(z,order,cap)
                actual = ho_phi(z,order,cap+1)
                expected = mp.hyp1f1(1,order+1,z)/mp.factorial(order)
                self.assertLess(abs(actual-expected),mp.mpf('1e-79'))

    def test_exact_residual_and_series_acceptance_boundaries(self) -> None:
        with mp.workdps(80):
            result = root(mp.mpf(27)/100,mp.mpf(0),7)
            self.assertEqual(abs(result.residual),16*mp.eps/128)
            z = -2*mp.eps/(1+mp.eps)
            self.assertEqual(ho_phi(z,1,2),1+z/2)
