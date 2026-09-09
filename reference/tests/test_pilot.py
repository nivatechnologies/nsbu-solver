"""Pilot admission, rollback and post-integration reference independence contracts."""
from contextlib import redirect_stdout
import io
import runpy
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.pilot import pilot, reservation, tracking
from reference.pilot_force import PilotForce
from reference.steps import RightHandSide
from tools.json_types import JsonObject, decode, object_value


class PilotTests(unittest.TestCase):
    def test_resource_and_method_admission_before_numerical_work(self) -> None:
        base,cache,total = reservation(4,8192,80,'CM',1024**3)
        self.assertEqual(cache,65*27*3*4096)
        self.assertIsInstance(cache,int)
        self.assertIsInstance(total,int)
        self.assertEqual(total,int(str(base['reserved_bytes']))+cache)
        self.assertEqual(reservation(4,8192,80,'CM',total)[2],total)
        for divisor,method in ((8191,'CM'),(8193,'HO'),(0,'CM'),(8192,'RK4')):
            with self.assertRaisesRegex(ValueError,'bounded timestep'):
                reservation(4,divisor,80,method,1024**3)
        with self.assertRaisesRegex(ValueError,'cache reservation'):
            reservation(4,8192,80,'CM',total-1)
        self.assertEqual(reservation(4,32768,120,'HO',1024**3)[1],257*27*3*4096)

    def exercise(self, method_name: str, divisor: int, bad_step: bool = False) -> JsonObject:
        calls: list[mpf] = []
        observed: list[Spectrum] = []
        def step(state: Spectrum, time: mpf, dt: mpf, _rhs: RightHandSide, nu: mpf) -> Spectrum:
            self.assertLess(len(calls),divisor//256)
            self.assertIsInstance(_rhs,PilotForce)
            assert isinstance(_rhs,PilotForce)
            self.assertEqual(_rhs.cache_slots,{8192:65,16384:129,32768:257}[divisor])
            self.assertEqual((_rhs.n,_rhs.precision,_rhs.cap_bytes),(4,80,1024**3))
            self.assertEqual(_rhs.dt,mp.mpf(1)/divisor)
            self.assertEqual(mp.dps,80)
            self.assertEqual(time,mp.mpf(len(calls))/divisor)
            self.assertEqual(dt,mp.mpf(1)/divisor)
            self.assertEqual(nu,mp.mpf(1))
            self.assertEqual(state[(0,0,0)][0],mp.mpc(len(calls)))
            if not calls:
                self.assertEqual(len(state),27)
                self.assertTrue(all(value == 0 for vector in state.values() for value in vector))
            calls.append(time)
            value = mp.mpc(mp.inf) if bad_step and len(calls) == 2 else mp.mpc(len(calls))
            return {k:(value,mp.mpc(0),mp.mpc(0)) for k in state}
        def reference(state: Spectrum, n: int, time: mpf) -> JsonObject:
            observed.append(state)
            self.assertEqual(n,4)
            self.assertEqual(time,mp.mpf(1 if bad_step else divisor//256)/divisor)
            self.assertEqual(len(calls),2 if bad_step else divisor//256)
            return {'post_integration':True}
        selected = 'cm_step' if method_name == 'CM' else 'ho_step'
        other = 'ho_step' if selected == 'cm_step' else 'cm_step'
        with patch('reference.pilot.'+other,side_effect=AssertionError('wrong integration method')),\
             patch('reference.pilot.'+selected,side_effect=step),\
             patch('reference.pilot.tracking',side_effect=reference):
            result = pilot(4,divisor,80,method_name,1024**3)
        self.assertEqual(len(observed),1)
        self.assertEqual(result['reference_assignments'],0)
        self.assertEqual(result['accepted_pde_windows'],0)
        self.assertEqual(object_value(result['preflight'])['cap_bytes'],1024**3)
        self.assertEqual(result['maximum_rhs_evaluations'],(4 if method_name == 'CM' else 5)*(divisor//256))
        self.assertEqual(result['method'],method_name)
        self.assertEqual(result['grid'],4)
        self.assertEqual(result['target'],'1/256')
        self.assertEqual(result['initial_condition'],'exact rest')
        self.assertEqual(result['quantum_exponent'],-20)
        self.assertEqual(result['precision'],80)
        self.assertEqual(result['divisor'],divisor)
        return result

    def test_fixed_clock_schedule_and_reference_only_after_evolution(self) -> None:
        for method,divisor in ((''.join(['C','M']),8192),(''.join(['H','O']),16384),('CM',32768)):
            result = self.exercise(method,divisor)
            self.assertEqual(result['status'],'diagnostic-completed')
            self.assertEqual(result['accepted_ticks'],4096)
            self.assertEqual(result['remaining_ticks'],4096)
            self.assertEqual(result['steps_completed'],divisor//256)
            self.assertIsInstance(result['steps_completed'],int)
            self.assertEqual(result['failure'],'')
            self.assertEqual(result['tracking_difference'],{'post_integration':True})

    def test_nonfinite_candidate_does_not_commit_time_or_coefficients(self) -> None:
        result = self.exercise('CM',8192,True)
        self.assertEqual(result['status'],'diagnostic-stopped')
        self.assertEqual(result['accepted_ticks'],128)
        self.assertEqual(result['remaining_ticks'],8064)
        self.assertEqual(result['steps_completed'],1)
        self.assertEqual(result['failure'],'Non-finite candidate refused')
        self.assertEqual(object_value(result['state'])['(0, 0, 0)'],[['1.0','0.0'],['0.0','0.0'],['0.0','0.0']])

    def test_rhs_and_reference_failures_preserve_diagnostic_evidence(self) -> None:
        with patch('reference.pilot.cm_step',side_effect=ValueError('force unresolved')),\
             patch('reference.pilot.ho_step',side_effect=AssertionError('wrong integration method')),\
             patch('reference.pilot.tracking',side_effect=ArithmeticError('reference unresolved')):
            result = pilot(4,8192,80,'CM',1024**3)
        self.assertEqual(result['failure'],'force unresolved')
        self.assertEqual(result['reference_failure'],'reference unresolved')
        self.assertEqual(result['accepted_ticks'],0)
        self.assertEqual(result['steps_completed'],0)
        self.assertEqual(result['rhs_evaluations'],0)
        self.assertEqual(result['force_sampling'],[])
        self.assertNotIn('tracking_difference',result)

    def test_tracking_signed_grid_and_noninteger_error(self) -> None:
        points: list[tuple[mpf,mpf,mpf]] = []
        def reference(x: mpf, y: mpf, z: mpf, time: mpf) -> tuple[tuple[mpf,mpf,mpf],mpf]:
            self.assertEqual(time,mp.mpf(1)/256)
            points.append((x,y,z))
            return (mp.mpf(1)/3,mp.mpf(0),mp.mpf(0)),mp.mpf(0)
        for n in (4,8):
            points.clear()
            with mp.workdps(80),patch('reference.pilot.fields',side_effect=reference):
                result = tracking({(0,0,0):(mp.mpc(1),mp.mpc(0),mp.mpc(0))},n,mp.mpf(1)/256)
            self.assertEqual(len(set(points)),n**3)
            self.assertEqual(min(p[0] for p in points),mp.mpf('-0.5'))
            self.assertEqual(max(p[2] for p in points),mp.mpf('0.25') if n == 4 else mp.mpf('0.375'))
            self.assertEqual(result['max_component_change'],'0.66666666666666666667')

    def test_module_entry_point_emits_diagnostic_json(self) -> None:
        count = 0
        def step(state: Spectrum, _time: mpf, _dt: mpf, _rhs: RightHandSide, _nu: mpf) -> Spectrum:
            nonlocal count
            self.assertLess(count,32)
            self.assertEqual(_time,mp.mpf(count)/8192)
            self.assertEqual((_dt,_nu,mp.dps),(mp.mpf(1)/8192,mp.mpf(1),80))
            count += 1
            return state
        stream = io.StringIO()
        with patch('reference.steps.cm_step',side_effect=step),\
             patch('reference.steps.ho_step',side_effect=AssertionError('wrong integration method')),\
             patch('reference.scalar.fields',return_value=((mp.mpf(0),)*3,mp.mpf(0))),redirect_stdout(stream):
            runpy.run_path('reference/pilot.py',run_name=''.join(['__','main__']))
        result = object_value(decode(stream.getvalue()))
        self.assertEqual(result['status'],'diagnostic-completed')
        self.assertEqual(result['accepted_ticks'],4096)
        self.assertEqual(result['accepted_pde_windows'],0)
        self.assertEqual(object_value(result['preflight'])['cap_bytes'],1024**3)
