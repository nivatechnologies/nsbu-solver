"""Physical normalization, resource bounds, and mutation-driven numerical contracts."""
from pathlib import Path
import unittest
from mpmath import mp, mpf
from reference.dft import Spectrum, preflight, retained
from reference.regions import coverage, radial_limit
from reference.scalar import CoordinateUnresolved, root, step_derivative
from reference.steps import cm_step, ho_step
from reference.tuples import triple
from reference.verify_steps import force, study
from reference.verify_trajectory import errors, prescribed_force, spatial_profile
from tools.json_types import Json, array_value, decode, object_value, string_value


class NumericalContracts(unittest.TestCase):
    def test_full_profile_and_source_at_nonzero_time(self) -> None:
        with mp.workdps(80):
            profile = spatial_profile()
            time = mp.mpf(1)/512
            expected: Spectrum = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in retained(4)}
            expected[(0,0,0)] = (mp.mpc(1),mp.mpc(-2),mp.mpc(3))
            for component,mode in enumerate(((0,1,0),(0,0,1),(1,0,0))):
                vector = [mp.mpc(0),mp.mpc(0),mp.mpc(0)]
                vector[component] = (component+1)*mp.mpc(0,-mp.mpf(1)/2)
                expected[mode] = triple(vector)
                expected[triple(-m for m in mode)] = triple(mp.conj(v) for v in vector)
            self.assertEqual(profile,expected)
            prescribed = force(4,time)
            for mode,vector in expected.items():
                factor = 1 if mode == (0,0,0) else 1+mp.sin(7*time)
                self.assertEqual(prescribed[mode],triple(factor*v for v in vector))
            quadratic: Spectrum = {k:(mp.mpc(2),mp.mpc(-3),mp.mpc(5)) for k in profile}
            result = prescribed_force(profile,quadratic,time)
            for mode,vector in result.items():
                laplacian = 4*mp.pi**2*sum(m*m for m in mode)
                for component,value in enumerate(vector):
                    expected_value = (13*mp.cos(13*time)+laplacian*mp.sin(13*time))*expected[mode][component]
                    expected_value -= mp.sin(13*time)**2*quadratic[mode][component]
                    self.assertLess(abs(value-expected_value),mp.mpf('1e-75'))

    def test_derivative_sensitive_error_norm(self) -> None:
        with mp.workdps(80):
            mode = (-2,1,0)
            state: Spectrum = {mode:(mp.mpc(3),mp.mpc(4),mp.mpc(0))}
            zero: Spectrum = {mode:(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            l2,h1 = errors(state,zero,mp.mpf(0))
            self.assertEqual(l2,5)
            self.assertLess(abs(h1-5*mp.sqrt(1+20*mp.pi**2)),mp.mpf('1e-75'))

    def test_high_mode_diffusion(self) -> None:
        with mp.workdps(80):
            mode = (2,-3,1)
            state: Spectrum = {mode:(mp.mpc(3),mp.mpc(2),mp.mpc(0))}
            def zero_rhs(current: Spectrum, _time: mpf) -> Spectrum:
                return {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in current}
            dt = mp.mpf(1)/1000
            for method in (cm_step,ho_step):
                result = method(state,mp.mpf(0),dt,zero_rhs,mp.mpf(2))
                expected = triple(v*mp.exp(-112*mp.pi**2*dt) for v in state[mode])
                self.assertLess(max(abs(a-b) for a,b in zip(result[mode],expected)),mp.mpf('1e-75'))

    def test_resource_ledger_exact_caps(self) -> None:
        for n in (4,8,12):
            retained_bytes = 786432*n**3
            padded_bytes = 294912*(3*n//2)**3
            total = retained_bytes+padded_bytes+67108864
            report = preflight(n,80,total)
            self.assertEqual(report['reserved_bytes'],total)
            allocations = object_value(report['allocations'])
            self.assertEqual(allocations,{'retained_vectors':retained_bytes,
                                         'padded_vectors_and_dft_temporaries':padded_bytes,
                                         'metadata_and_interpreter_reservation':67108864})
            self.assertEqual(len(retained(n)),(n-1)**3)
            with self.assertRaisesRegex(ValueError,'reservation cap'):
                preflight(n,120,total-1)
        with self.assertRaises(ValueError):
            retained(13)

    def test_noncentral_region_geometry_and_touching_empty(self) -> None:
        with mp.workdps(80):
            tau = mp.mpf(1)/128
            eta = mp.mpf(1)/2
            q = mp.mpf(1)/96
            radius = mp.mpf(3)/10
            limit,weight = radial_limit(eta,tau,radius)
            self.assertLess(abs(limit-(radius**2-q**(mp.mpf(3)/4)/4)/(2*q)),mp.mpf('1e-75'))
            self.assertLess(abs(weight-q**(mp.mpf(11)/8)*mp.mpf(5)/4),mp.mpf('1e-75'))
            result = coverage(tau,radius**2/(2*tau),mp.mpf(8),radius,2)
            self.assertEqual(result.status,'RegionEmpty')
            self.assertEqual(result.fraction,0)

    def test_root_exact_work_boundary_and_flat_derivative(self) -> None:
        with mp.workdps(80):
            z,time = mp.mpf(1)/8,mp.mpf(1)/512
            solved = root(z,time)
            self.assertEqual(root(z,time,solved.iterations),solved)
            with self.assertRaises(CoordinateUnresolved):
                root(z,time,solved.iterations-1)
            for argument in (-2,0,1,2):
                self.assertEqual(step_derivative(mp.mpf(argument)),0)

    def test_actual_step_fixture_discrepancy_and_mean(self) -> None:
        fixture = object_value(decode((Path(__file__).resolve().parents[2]/'fixtures/reference/steps-n4.json').read_text()))
        rows = [object_value(row) for row in array_value(fixture['studies'])]
        expected = {string_value(row['method']):row for row in rows if row['precision'] == 80}
        studies = study(4,80)
        self.assertEqual([item.method for item in studies],["CM","HO"])
        for item in studies:
            self.assertEqual(item.preflight['cap_bytes'],4*1024**3)
            with mp.workdps(80):
                self.assertLess(max(abs(a-b) for a,b in zip(item.full_step[(0,0,0)],
                                                         (mp.mpf(1)/100,-mp.mpf(2)/100,mp.mpf(3)/100))),mp.mpf('1e-75'))
                differences = [abs(a-b) for k in item.full_step for a,b in zip(item.full_step[k],item.two_half_steps[k])]
                self.assertEqual(item.raw_local_discrepancy,max(differences))
                self.assertGreater(item.raw_local_discrepancy,0)
                for path,actual in (('full_step',item.full_step),('two_half_steps',item.two_half_steps)):
                    self.assert_spectrum_fixture(actual,expected[item.method][path])

    def assert_spectrum_fixture(self, actual: Spectrum, encoded_state: Json) -> None:
        for encoded in array_value(encoded_state):
            entry = object_value(encoded)
            mode = triple(int(str(v)) for v in array_value(entry['mode']))
            values = [array_value(v) for v in array_value(entry['value'])]
            for component,pair in zip(actual[mode],values):
                stored = mp.mpc(mp.mpf(string_value(pair[0])),mp.mpf(string_value(pair[1])))
                self.assertLess(abs(component-stored),mp.mpf('1e-60'))
