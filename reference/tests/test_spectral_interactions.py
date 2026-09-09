"""Overlapping wavevectors and a retained-band quadratic alias negative control."""
import unittest
from mpmath import mp
from reference.dft import (Spectrum, convolution, cross, curl, forward_vector,
                           inverse_vector, nonlinear, preflight, project, transform)
from reference.tuples import triple


class SpectralInteractionTests(unittest.TestCase):
    def test_overlapping_wavevector_convolution(self) -> None:
        with mp.workdps(80):
            target = (2,1,1)
            state: Spectrum = {target:(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            for mode,vector in (((1,1,0),(1,-1,0)),((1,0,1),(1,0,-1))):
                state[mode] = triple(mp.mpc(v) for v in vector)
                state[triple(-m for m in mode)] = triple(mp.mpc(v) for v in vector)
            actual = convolution(state)[target]
            expected = triple(mp.mpc(0,s*8*mp.pi/3) for s in (-1,1,1))
            self.assertLess(max(abs(a-b) for a,b in zip(actual,expected)),mp.mpf('1e-75'))

    def test_actual_quadratic_padding_removes_retained_alias(self) -> None:
        preflight(8,80,2*1024**3)
        with mp.workdps(80):
            state: Spectrum = {}
            for mode,vector in (((3,1,0),(1,-3,0)),((3,0,1),(1,0,-3))):
                state[mode] = triple(mp.mpc(v) for v in vector)
                state[triple(-m for m in mode)] = triple(mp.mpc(v) for v in vector)
            alias = (-2,1,1)
            corrected = nonlinear(state,8)
            self.assertLess(max(abs(v) for v in corrected[alias]),mp.mpf('1e-73'))
            velocity = inverse_vector(state,8)
            vorticity = inverse_vector(curl(state),8)
            unpadded = forward_vector([cross(u,w) for u,w in zip(velocity,vorticity)],8,8)
            corrupted = project(alias,unpadded[alias])
            self.assertGreater(max(abs(v) for v in corrupted),1)

    def test_largest_padded_transform_grid(self) -> None:
        preflight(12,80,4*1024**3)
        with mp.workdps(80):
            values = [mp.mpc(0)]*(18**3)
            values[0] = mp.mpc(1)
            coefficients = transform(values,18)
            self.assertLess(max(abs(v-mp.mpf(1)/(18**3)) for v in coefficients),mp.mpf('1e-75'))
            with self.assertRaisesRegex(ValueError,'Invalid direct-DFT grid'):
                transform([mp.mpc(0)]*(17**3),17)

    def test_overlong_transform_payload_is_refused(self) -> None:
        with self.assertRaisesRegex(ValueError,'Invalid direct-DFT grid'):
            transform([mp.mpc(0)]*65,4)
