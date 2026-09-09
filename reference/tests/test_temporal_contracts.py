"""State-dependent stages, nonautonomous order and coefficient conditioning."""
import unittest
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.steps import cm_step, ho_step, cm_weights, ho_phi, ho_tableau, coefficient_guard_digits
from reference.tuples import triple


class TemporalContracts(unittest.TestCase):
    def setUp(self) -> None:
        context = mp.workdps(80)
        context.__enter__()
        self.addCleanup(context.__exit__,None,None,None)

    def test_zero_operator_stability_polynomial(self) -> None:
        initial: Spectrum = {(0,0,0):(mp.mpc(1),mp.mpc(2),mp.mpc(3))}
        def rhs(state: Spectrum, _time: mpf) -> Spectrum:
            return dict(state)
        dt = mp.mpf(1)/7
        expected = sum(dt**j/mp.factorial(j) for j in range(5))
        for method in (cm_step,ho_step):
            computed = method(initial,mp.mpf(0),dt,rhs,mp.mpf(1))
            for i in range(3):
                self.assertLess(abs(computed[(0,0,0)][i]-(i+1)*expected),mp.mpf('1e-75'))

    def test_nonautonomous_uniform_ode_order(self) -> None:
        def rhs(state: Spectrum, time: mpf) -> Spectrum:
            value = state[(0,0,0)]
            forcing = 7*mp.cos(7*time)+3*mp.sin(7*time)
            return {(0,0,0):triple(-3*v+forcing for v in value)}
        for method in (cm_step,ho_step):
            errors: list[mpf] = []
            for count in (2,4,8,16):
                dt = mp.mpf(1)/(10*count)
                state: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
                for i in range(count):
                    state = method(state,i*dt,dt,rhs,mp.mpf(1))
                errors.append(abs(state[(0,0,0)][0]-mp.sin(mp.mpf(7)/10)))
            self.assertGreater(errors[-2]/errors[-1],14)
            self.assertLess(errors[-2]/errors[-1],18)

    def test_extreme_weight_conditioning(self) -> None:
        for text in ('0','-1e-30','-0.999999999999','-1.000000000001','-49.999999999','-50.000000001','-1000','-1e308'):
            z = mp.mpf(text)
            actual = cm_weights(z)
            _,ho = ho_tableau(z)
            self.assertLess(abs(actual[1]-ho[0]),mp.mpf('1e-70'))
            self.assertLess(abs(actual[2]*2-ho[4]),mp.mpf('1e-70'))
            self.assertLess(abs(actual[3]-ho[3]),mp.mpf('1e-70'))
        z = -mp.mpf('1e308')
        q,w1,w2,w3 = cm_weights(z)
        self.assertLess(abs(w1*z*z+1),mp.mpf('1e-70'))
        self.assertLess(abs(w2*z*z-2),mp.mpf('1e-70'))
        self.assertLess(abs(w3*z+1),mp.mpf('1e-70'))
        self.assertLess(abs(q*z+1),mp.mpf('1e-70'))

    def test_coefficient_refusals(self) -> None:
        for z in (mp.inf,mp.nan,mp.mpf(1),-mp.mpf('1e310')):
            with self.assertRaises(ValueError):
                coefficient_guard_digits(z)
        for order,cap in ((0,512),(4,512),(1,0)):
            with self.assertRaises(ValueError):
                ho_phi(mp.mpf(-1),order,cap)
