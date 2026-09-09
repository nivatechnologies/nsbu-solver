"""Joint cutoff/startup and nonzero-operator stage contracts."""
import unittest
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.evaluator import evaluate, smooth_step
from reference.jets import Jet
from reference.scalar import fields, root
from reference.steps import cm_step, ho_step
from reference.tuples import triple
from reference.verify_fields import scalar_force


class CoupledLimitTests(unittest.TestCase):
    def test_three_dimensional_collar_during_startup(self) -> None:
        with mp.workdps(80):
            point = (mp.mpf(1)/5,mp.mpf(1)/4,mp.mpf(3)/20,mp.mpf(1)/1024)
            jet = evaluate(*point)
            velocity,pressure = fields(*point)
            self.assertLess(max(abs(a-b) for a,b in zip(jet.velocity,velocity)),mp.mpf('1e-75'))
            self.assertLess(abs(jet.pressure_raw-pressure),mp.mpf('1e-75'))
            force = scalar_force(point)
            self.assertLess(max(abs(a-b) for a,b in zip(jet.force,force)),mp.mpf('1e-70'))
            self.assertGreater(max(abs(v) for v in force),1)

    def test_remaining_time_scaled_residual_and_nonfinite_input(self) -> None:
        with mp.workdps(80):
            time = mp.mpf(1)/128-mp.mpf('1e-40')
            solved = root(mp.mpf(1)/8,time,64)
            self.assertLessEqual(abs(solved.residual),16*mp.eps*(mp.mpf(1)/128-time))
            for invalid in (mp.nan,mp.inf,-mp.inf):
                with self.assertRaisesRegex(ValueError,'Require finite'):
                    root(invalid,mp.mpf(0),8)

    def test_nonautonomous_stages_with_nonzero_diffusion(self) -> None:
        with mp.workdps(80):
            mode = (1,0,0)
            frequency = mp.mpf(7)
            diffusion = 4*mp.pi**2
            def rhs(state: Spectrum, time: mpf) -> Spectrum:
                value = frequency*mp.cos(frequency*time)+(diffusion+3)*mp.sin(frequency*time)
                return {mode:triple(-3*v+value for v in state[mode])}
            for method in (cm_step,ho_step):
                errors: list[mpf] = []
                for steps in (8,16,32):
                    dt = mp.mpf(1)/(10*steps)
                    state: Spectrum = {mode:(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
                    for tick in range(steps):
                        state = method(state,tick*dt,dt,rhs,mp.mpf(1))
                    errors.append(abs(state[mode][0]-mp.sin(mp.mpf(7)/10)))
                self.assertGreater(errors[-2]/errors[-1],14)
                self.assertLess(errors[-2]/errors[-1],18)
                self.assertLess(errors[-1],mp.mpf('1e-7'))

    def test_cutoff_derivatives_close_to_both_flat_regions(self) -> None:
        with mp.workdps(80):
            for location in (mp.mpf('1e-10'),1-mp.mpf('1e-10')):
                value = smooth_step(Jet.variable(location,0))
                expected = 0 if location < mp.mpf(1)/2 else 1
                self.assertLess(abs(value.value-expected),mp.mpf('1e-70'))
                for order in range(1,5):
                    self.assertLess(abs(value.partial((order,0,0,0))),mp.mpf('1e-60'))
