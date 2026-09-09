"""Pure nonmonotone force requests, explicit work caps and exact dyadic conversion."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.pilot_force import PilotForce
from tools.json_types import JsonObject


class PilotForceTests(unittest.TestCase):
    def test_nonmonotone_requests_reuse_only_the_pure_force(self) -> None:
        seen: list[mpf] = []
        states: list[Spectrum] = []
        source: Spectrum = {(1,0,0):(mp.mpc(9),mp.mpc(2),mp.mpc(3))}
        def sampled(n: int, time: mpf, precision: int, cap: int) -> tuple[Spectrum,JsonObject]:
            self.assertEqual((n,precision,cap),(4,80,1024**3))
            self.assertEqual(mp.dps,80)
            seen.append(time)
            return source,{'time':str(time)}
        def nonlinear(state: Spectrum, n: int) -> Spectrum:
            self.assertEqual(n,4)
            states.append(state)
            return state
        with mp.workdps(80):
            rhs = PilotForce(4,80,1024**3,mp.mpf(1)/8192,3,2)
            first: Spectrum = {(1,0,0):(mp.mpc(0),mp.mpc(1),mp.mpc(2))}
            second: Spectrum = {(1,0,0):(mp.mpc(0),mp.mpc(4),mp.mpc(5))}
            with patch('reference.pilot_force.sample_force',side_effect=sampled),\
                 patch('reference.pilot_force.nonlinear',side_effect=nonlinear),\
                 patch('reference.pilot_force.inverse_vector',return_value=[(mp.mpc(0),)*3]):
                a = rhs(first,mp.mpf(1)/256)
                b = rhs(second,mp.mpf(1)/512)
                c = rhs(second,mp.mpf(1)/256)
            self.assertEqual(seen,[mp.mpf(1)/256,mp.mpf(1)/512])
            self.assertEqual(states,[first,second,second])
            self.assertEqual(a[(1,0,0)],(mp.mpc(0),mp.mpc(3),mp.mpc(5)))
            self.assertEqual(b[(1,0,0)],(mp.mpc(0),mp.mpc(6),mp.mpc(8)))
            self.assertEqual(c,b)
            self.assertEqual(rhs.evaluations,3)
            self.assertEqual(len(rhs.reports),2)
            self.assertEqual(len(rhs.cache),2)

    def test_stage_and_work_limits_are_refused(self) -> None:
        with mp.workdps(80):
            rhs = PilotForce(4,80,1024**3,mp.mpf(1)/8192,1,2)
            self.assertEqual(rhs.stage_tick(mp.mpf(0)),0)
            self.assertEqual(rhs.stage_tick(mp.mpf(1)/256),4096)
            for time in (mp.mpf(-1)/2**20,mp.mpf(-1)/2**21,mp.mpf(4097)/2**20,mp.mpf(1)/2**21):
                with self.assertRaisesRegex(ArithmeticError,'bound violated'):
                    rhs.stage_tick(time)
            for time in (mp.inf,mp.nan):
                with self.assertRaisesRegex(ArithmeticError,'not finite'):
                    rhs.stage_tick(time)
            rhs.evaluations = 1
            self.assertEqual(rhs.stage_tick(mp.mpf(0)),0)
            rhs.evaluations = 2
            with self.assertRaisesRegex(ArithmeticError,'bound violated'):
                rhs.stage_tick(mp.mpf(0))

    def test_advective_and_cache_limits_precede_new_force_work(self) -> None:
        with mp.workdps(80):
            state: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            dt = mp.mpf(1)/8192
            rhs = PilotForce(4,80,1024**3,dt,10,0)
            with patch('reference.pilot_force.inverse_vector',return_value=[(mp.mpc(0),)*3]):
                with self.assertRaisesRegex(ArithmeticError,'cache exceeded'):
                    rhs(state,mp.mpf(0))
            with patch('reference.pilot_force.inverse_vector',return_value=[(mp.mpc(1000),)*3]):
                with self.assertRaisesRegex(ArithmeticError,'Advective guard'):
                    rhs(state,mp.mpf(0))
            self.assertEqual(rhs.evaluations,2)
            self.assertEqual(rhs.cache,{})

    def test_conversion_failure_is_not_repaired_by_subtracting_rounded_time(self) -> None:
        with mp.workdps(80):
            rhs = PilotForce(4,80,1024**3,mp.mpf(1)/8192,10,1)
            original = mp.mpf
            for change in (-1,1):
                def corrupted(value: int) -> mpf:
                    return original(value+change if value == 8192 else value)
                with patch.object(mp,'mpf',side_effect=corrupted):
                    with self.assertRaisesRegex(ArithmeticError,'lost exact remaining'):
                        rhs.stage_tick(original(1)/2**20)

    def test_guard_uses_padded_grid_and_sum_of_absolute_components(self) -> None:
        with mp.workdps(80):
            state: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            for n in (4,8,12):
                for target in ('0.299999','0.300001'):
                    dt = mp.mpf(1)/8192
                    amplitude = mp.mpf(target)/(dt*2*mp.pi*(n//2-1)*6)
                    def inverse(current: Spectrum, padded: int) -> list[tuple[mpf,mpf,mpf]]:
                        self.assertIs(current,state)
                        self.assertEqual(padded,{4:6,8:12,12:18}[n])
                        self.assertIsInstance(padded,int)
                        return [(mp.mpf(0),mp.mpf(0),mp.mpf(0)),(-amplitude,2*amplitude,-3*amplitude)]
                    rhs = PilotForce(n,80,1024**3,dt,1,1)
                    rhs.cache[0] = state
                    with patch('reference.pilot_force.inverse_vector',side_effect=inverse),\
                         patch('reference.pilot_force.nonlinear',return_value=state):
                        if target == '0.299999':
                            self.assertEqual(rhs(state,mp.mpf(0)),state)
                        else:
                            with self.assertRaisesRegex(ArithmeticError,'Advective guard'):
                                rhs(state,mp.mpf(0))

    def test_exact_advective_boundary_is_accepted(self) -> None:
        with mp.workdps(80):
            state: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            rhs = PilotForce(4,80,1024**3,mp.mpf(1),1,1)
            rhs.cache[0] = state
            velocity = [(mp.mpc('-0.3'),mp.mpc(0),mp.mpc(0))]
            with patch.object(mp,'pi',mp.mpf('0.5')),\
                 patch('reference.pilot_force.inverse_vector',return_value=velocity),\
                 patch('reference.pilot_force.nonlinear',return_value=state):
                self.assertEqual(rhs(state,mp.mpf(0)),state)

    def test_overfull_cache_refuses_before_sampling(self) -> None:
        with mp.workdps(80):
            state: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            rhs = PilotForce(4,80,1024**3,mp.mpf(1)/8192,2,0)
            rhs.cache[1] = state
            with patch('reference.pilot_force.inverse_vector',return_value=[(mp.mpc(0),)*3]),\
                 patch('reference.pilot_force.sample_force',side_effect=AssertionError('sampling before refusal')):
                with self.assertRaisesRegex(ArithmeticError,'cache exceeded'):
                    rhs(state,mp.mpf(0))
