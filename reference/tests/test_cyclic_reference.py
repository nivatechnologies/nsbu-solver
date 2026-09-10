"""Exact force-bit admission and independently constructed smooth coefficients."""
import json
import unittest
from mpmath import mp
from reference.cyclic.bits import FixedForcing, binary64, mode_at
from reference.cyclic.profile import profile, projected_force
from reference.cyclic.study import StudyPlan, StudyRhs, maximum_difference
from reference.dft import retained
from tools.json_types import JsonObject


def fixture(n: int = 4) -> JsonObject:
    rows: list[JsonObject] = [
        {'mode': list(mode_at(i, n)), 'bits': [[0, 0], [0, 0], [0, 0]]}
        for i in range(n*n*(n//2+1))
    ]
    return {'grid': n, 'tick_exponent': -16,
            'samples': [{'tick': tick, 'force': rows} for tick in range(0,129,4)]}


def encode(data: JsonObject) -> bytes:
    return json.dumps(data).encode()


class CyclicReferenceTests(unittest.TestCase):
    def test_canonical_layout_uses_positive_nyquist_and_signed_wrapped_coordinates(self) -> None:
        self.assertEqual([mode_at(i,4) for i in (0,1,2,6,11,24,36,47)],
                         [(0,0,0),(0,0,1),(0,0,2),(0,2,0),(0,-1,2),
                          (2,0,0),(-1,0,0),(-1,-1,2)])

    def test_binary64_is_exact_even_when_caller_precision_is_low(self) -> None:
        with mp.workdps(5):
            values = [binary64(word) for word in (0, 1 << 63, 1, (1 << 52)-1,
                      (1023 << 52)+1, (2046 << 52)+((1 << 52)-1))]
        with mp.workdps(120):
            expected = [mp.mpf(0), mp.mpf(0), mp.mpf(2)**-1074,
                        ((1 << 52)-1)*mp.mpf(2)**-1074,
                        1+mp.mpf(2)**-52, ((1 << 53)-1)*mp.mpf(2)**971]
            self.assertEqual(values, expected)
        for word in (-1, 1 << 64, 2047 << 52, (2047 << 52)+1):
            with self.assertRaises(ValueError):
                binary64(word)

    def test_fixture_is_immutable_and_all_exact_stage_times_are_present(self) -> None:
        raw = encode(fixture())
        forcing = FixedForcing(raw, 4, len(raw))
        self.assertEqual(forcing.n, 4)
        self.assertEqual(len(forcing.sha256), 64)
        self.assertEqual(tuple(forcing.samples), tuple(range(0,129,4)))
        for tick in (0, 4, 128):
            values = forcing.evaluate(mp.mpf(tick)/65536)
            self.assertEqual(set(values), set(retained(4)))
            self.assertTrue(all(v == 0 for vector in values.values() for v in vector))
        for time in (mp.mpf(-1), mp.inf, mp.mpf(129)/65536,
                     mp.mpf(1)/65536, mp.mpf(1)/131072):
            with self.assertRaises(ValueError):
                forcing.evaluate(time)
        with self.assertRaises(ValueError):
            FixedForcing(raw, 4, len(raw)-1)
        with self.assertRaises(ValueError):
            FixedForcing(raw, 8, len(raw))
        with self.assertRaises(ValueError):
            FixedForcing(raw, 3, len(raw))

    def test_independent_profile_force_has_the_correct_startup_and_quadratic_coupling(self) -> None:
        with mp.workdps(80):
            state, derivative = profile(4, mp.mpf(0))
            self.assertTrue(all(v == 0 for vector in state.values() for v in vector))
            force = projected_force(4, mp.mpf(0))
            self.assertEqual(force, derivative)
            self.assertEqual(derivative[(0,1,0)][0], -13*mp.j/2)
            time = mp.mpf(1)/512
            force = projected_force(4, time)
            self.assertNotEqual(force[(0,1,1)][0], 0)
            self.assertLess(abs(force[(0,1,1)][0] + mp.j*mp.pi*mp.sin(13*time)*mp.sin(17*time)/2), mp.mpf('1e-75'))
        for time in (mp.mpf(-1), mp.inf):
            with self.assertRaises(ValueError):
                profile(4, time)

    def test_work_and_shape_refusals_preserve_spent_rhs_allowances(self) -> None:
        self.assertEqual(StudyPlan(4,80,'CM',4*1024**3).maximum_calls(),96)
        self.assertEqual(StudyPlan(4,120,'HO',4*1024**3).maximum_calls(),120)
        with self.assertRaises(ValueError):
            StudyPlan(4,80,'bad',4*1024**3).reservation()
        with self.assertRaises(ValueError):
            StudyPlan(4,80,'CM',1).reservation()
        rhs = StudyRhs(4,2,lambda time: projected_force(4,time))
        with self.assertRaises(ArithmeticError):
            rhs({},mp.mpf(0))
        self.assertEqual(rhs.calls,1)
        with self.assertRaises(ArithmeticError):
            rhs({},mp.mpf(1)/65536)
        self.assertEqual(rhs.calls,2)
        with self.assertRaises(ArithmeticError):
            rhs({},mp.mpf(0))
        for left, right in (({}, {}), ({(0,0,0):(mp.mpc(mp.inf),mp.mpc(0),mp.mpc(0))},
                                     {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))})):
            with self.assertRaises((ValueError,ArithmeticError)):
                maximum_difference(left,right)


class CyclicFixtureFailures(unittest.TestCase):
    def test_schema_order_words_nyquist_and_reality_failures_are_explicit(self) -> None:
        from tools.json_types import Json, array_value, object_value
        changes: list[tuple[str, str, Json]] = [
            ('root', 'grid', True), ('root', 'tick_exponent', -15),
            ('root', 'samples', []), ('time', 'tick', 4),
            ('time', 'force', []), ('mode', 'mode', [1,0,0]),
            ('mode', 'bits', [[0,0],[0,0]]),
            ('mode', 'bits', [[0],[0,0],[0,0]]),
            ('mode', 'bits', [[0.5,0],[0,0],[0,0]]),
            ('mode', 'bits', [[1 << 64,0],[0,0],[0,0]]),
            ('mode', 'bits', [[2047 << 52,0],[0,0],[0,0]]),
            ('mode', 'bits', [[0,1023 << 52],[0,0],[0,0]]),
        ]
        for scope, key, value in changes:
            data = fixture()
            times = [dict(object_value(v)) for v in array_value(data['samples'])]
            rows = [dict(object_value(v)) for v in array_value(times[0]['force'])]
            data['samples'] = times
            times[0]['force'] = rows
            target = data if scope == 'root' else times[0] if scope == 'time' else rows[0]
            target[key] = value
            raw = encode(data)
            with self.subTest(scope=scope,key=key,value=value), self.assertRaises(ValueError):
                FixedForcing(raw,4,len(raw))
        for index in (2,12):
            data = fixture()
            times = [dict(object_value(v)) for v in array_value(data['samples'])]
            rows = [dict(object_value(v)) for v in array_value(times[0]['force'])]
            data['samples'] = times
            times[0]['force'] = rows
            rows[index]['bits'] = [[1023 << 52,0],[0,0],[0,0]]
            raw = encode(data)
            with self.assertRaises(ValueError):
                FixedForcing(raw,4,len(raw))

    def test_nonzero_conjugate_pair_and_signed_zero_decode_without_loss(self) -> None:
        from tools.json_types import array_value, object_value
        data = fixture()
        times = [dict(object_value(v)) for v in array_value(data['samples'])]
        rows = [dict(object_value(v)) for v in array_value(times[0]['force'])]
        data['samples'] = times
        times[0]['force'] = rows
        amplitude_bits = (1026 << 52)+(3 << 48)  # exact 19/2
        rows[12]['bits'] = [[0,0],[0,0],[0,amplitude_bits+(1 << 63)]]
        rows[36]['bits'] = [[0,0],[0,0],[0,amplitude_bits]]
        rows[2]['bits'] = [[1 << 63,0],[0,1 << 63],[0,0]]
        raw = encode(data)
        values = FixedForcing(raw,4,len(raw)).evaluate(mp.mpf(0))
        self.assertEqual(values[(1,0,0)][2],-mp.j*19/2)
        self.assertEqual(values[(-1,0,0)][2],mp.j*19/2)
