"""Direct accumulation keeps small DC contributions amid severe cancellation."""
import unittest
from mpmath import mp
from reference.dft import transform


class DirectAccumulationTests(unittest.TestCase):
    def test_direct_dc_sum_preserves_a_small_term_between_large_opposite_inputs(self) -> None:
        with mp.workdps(80):
            values = [mp.mpc(0)]*64
            values[0] = mp.mpc('1e100')
            values[1] = mp.mpc(1)
            values[2] = mp.mpc('-1e100')
            output = transform(values,4)
            self.assertEqual(output[0],mp.mpc(mp.mpf(1)/64))
            self.assertEqual(len(output),64)

    def test_a_known_complex_mode_round_trips_with_all_modes_retained(self) -> None:
        with mp.workdps(80):
            spectrum = [mp.mpc(0)]*64
            spectrum[1] = mp.mpc(3,2)
            physical = transform(spectrum,4,True)
            reconstructed = transform(physical,4)
            self.assertLess(max(abs(a-b) for a,b in zip(spectrum,reconstructed)),mp.mpf('1e-75'))
