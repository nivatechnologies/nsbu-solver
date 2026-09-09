"""Independent transform, convolution and step contract fixtures."""
import unittest
from mpmath import mp
from reference.dft import (convolution, curl, grid, inverse_vector, nonlinear,
                           preflight, project, retained, transform)
from reference.steps import cm_step, ho_step, ho_tableau, ho_phi


def smooth_state(n: int) -> dict:
    state = {k:(mp.mpc(0),)*3 for k in retained(n)}
    for mode,component,amplitude in (((0,1,0),0,1),((0,0,1),1,2),((1,0,0),2,3)):
        vector = [mp.mpc(0)]*3
        vector[component] = amplitude/(2*mp.j)
        state[mode] = tuple(vector)
        state[tuple(-k for k in mode)] = tuple(mp.conj(v) for v in vector)
    return state


class DftTests(unittest.TestCase):
    def setUp(self) -> None:
        preflight(4,80,1024**3)
        context = mp.workdps(80)
        context.__enter__()
        self.addCleanup(context.__exit__,None,None,None)

    def test_direct_transform_mode_and_roundtrip(self) -> None:
        values = [mp.exp(2*mp.pi*mp.j*(x-y)/4)+3 for x,y,z in grid(4)]
        coefficients = transform(values,4)
        self.assertLess(abs(coefficients[0]-3),mp.mpf('1e-75'))
        self.assertLess(abs(coefficients[(1*4+3)*4]-1),mp.mpf('1e-75'))
        recovered = transform(coefficients,4,True)
        self.assertLess(max(abs(a-b) for a,b in zip(values,recovered)),mp.mpf('1e-75'))

    def test_product_matches_convolution_and_energy(self) -> None:
        state = smooth_state(4)
        direct = nonlinear(state,4)
        explicit = convolution(state)
        error = max(abs(a-b) for k in state for a,b in zip(direct[k],explicit[k]))
        self.assertLess(error,mp.mpf('1e-74'))
        energy = sum(mp.re(mp.conj(u)*v) for k in state for u,v in zip(state[k],direct[k]))
        self.assertLess(abs(energy),mp.mpf('1e-74'))

    def test_gradient_projection_mean_and_curl(self) -> None:
        self.assertEqual(project((1,-2,3),(mp.mpc(1),mp.mpc(-2),mp.mpc(3))),(0,0,0))
        mean = (mp.mpc(1),mp.mpc(2),mp.mpc(3))
        self.assertEqual(project((0,0,0),mean),mean)
        self.assertEqual(curl({(0,0,0):mean})[(0,0,0)],(0,0,0))

    def test_alias_negative_control(self) -> None:
        # A true mode 3 aliases to retained mode -1 on N=4.
        values = [mp.exp(2*mp.pi*mp.j*3*x/4) for x,y,z in grid(4)]
        self.assertLess(abs(transform(values,4)[3*16]-1),mp.mpf('1e-75'))
        padded = [mp.exp(2*mp.pi*mp.j*3*x/6) for x,y,z in grid(6)]
        self.assertLess(abs(transform(padded,6)[5*36]),mp.mpf('1e-75'))

    def test_preflight_refusals(self) -> None:
        with self.assertRaises(ValueError):
            preflight(4,80,1)
        with self.assertRaises(ValueError):
            preflight(16,80,4*1024**3)
        self.assertLess(preflight(4,80,4*1024**3)['reserved_bytes'],4*1024**3)

    def test_actual_stage_times_and_constant_source(self) -> None:
        state = {(0,0,0):(mp.mpc(1),mp.mpc(2),mp.mpc(3))}
        source = {(0,0,0):(mp.mpc(2),mp.mpc(-1),mp.mpc(4))}
        dt = mp.mpf('0.125')
        for method,expected in ((cm_step,(0,dt/2,dt/2,dt)),
                                (ho_step,(0,dt/2,dt/2,dt,dt/2))):
            requested = []
            def rhs(current: dict, time: mp.mpf) -> dict:
                requested.append(time)
                return source
            result = method(state,mp.mpf(0),dt,rhs,mp.mpf(1))
            self.assertEqual(tuple(requested),expected)
            self.assertLess(max(abs(a+dt*b-c) for a,b,c in
                                zip(state[(0,0,0)],source[(0,0,0)],result[(0,0,0)])),mp.mpf('1e-75'))

    def test_modal_diffusion(self) -> None:
        state = {(1,0,0):(mp.mpc(0),mp.mpc(1),mp.mpc(2))}
        for method in (cm_step,ho_step):
            result = method(state,mp.mpf(0),mp.mpf('0.01'),
                            lambda u,t: {k:(mp.mpc(0),)*3 for k in u},mp.mpf(1))
            expected = mp.exp(-4*mp.pi**2/100)
            self.assertLess(abs(result[(1,0,0)][1]-expected),mp.mpf('1e-75'))

    def test_nonzero_ho_rows(self) -> None:
        for z in (mp.mpf('-0.1'),mp.mpf(-1),mp.mpf(-10)):
            rows,weights = ho_tableau(z)
            nodes = (0,mp.mpf('0.5'),mp.mpf('0.5'),1,mp.mpf('0.5'))
            for node,row in zip(nodes[1:],rows[1:]):
                self.assertLess(abs(sum(row)-node*ho_phi(node*z,1)),mp.mpf('1e-70'))
            self.assertLess(abs(sum(weights)-ho_phi(z,1)),mp.mpf('1e-70'))


if __name__ == '__main__':
    unittest.main()
