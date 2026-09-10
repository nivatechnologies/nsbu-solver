"""Independent physical-wave derivatives, complete tensor order and pressure-band controls."""
from itertools import product
import unittest
from mpmath import mp, mpf
from reference.dft import Spectrum, retained
from reference.derived.spectral import fields, index, inventory, inverse, modes, pressure
from reference.tuples import triple


def taylor_green() -> Spectrum:
    state = {k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in retained(4)}
    for a,b in product((-1,1),repeat=2):
        state[(a,b,0)] = (mp.mpc(0,-mp.mpf(a)/4),mp.mpc(0,mp.mpf(b)/4),mp.mpc(0))
    return state


def exact(quantity: str, component: int, x: mpf, y: mpf, _z: mpf) -> mpf:
    if quantity=='pressure':
        return (mp.cos(4*mp.pi*x)+mp.cos(4*mp.pi*y))/4
    if quantity=='vorticity':
        return 4*mp.pi*mp.sin(2*mp.pi*x)*mp.sin(2*mp.pi*y) if component==2 else mp.mpf(0)
    if component==0:
        return mp.sin(2*mp.pi*x)*mp.cos(2*mp.pi*y)
    return -mp.cos(2*mp.pi*x)*mp.sin(2*mp.pi*y) if component==1 else mp.mpf(0)


class DerivedSpectralTests(unittest.TestCase):
    def test_pressure_and_all_ordered_fields_match_independent_physical_derivatives(self) -> None:
        with mp.workdps(80):
            state=taylor_green()
            force={k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in modes(8)}
            p,excluded=pressure(state,force,4)
            expected={k:mp.mpc(0) for k in modes(8)}
            for k in ((2,0,0),(-2,0,0),(0,2,0),(0,-2,0)):
                expected[k]=mp.mpc(mp.mpf(1)/8)
            self.assertLess(max(abs(p[k]-v) for k,v in expected.items()),mp.mpf('1e-75'))
            self.assertEqual(p[(0,0,0)],0)
            self.assertLess(excluded,mp.mpf('1e-75'))
            self.assertEqual(len(inventory()),46)
            for (quantity,component,orders),values in zip(inventory(),fields(state,p,4),strict=True):
                for point in ((0,0,0),(1,2,3),(3,1,2)):
                    xyz=triple(mp.mpf(k)/8 for k in point)
                    def function(x:mpf,y:mpf,z:mpf)->mpf:
                        return exact(quantity,component,x,y,z)
                    self.assertLess(abs(values[index(point,8)]-mp.diff(function,xyz,orders)),mp.mpf('1e-73'))

    def test_high_force_mode_is_preserved_and_complex_reality_defects_are_visible(self) -> None:
        with mp.workdps(80):
            state={k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in retained(4)}
            force={k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in modes(8)}
            force[(3,0,0)]=force[(-3,0,0)]=(mp.mpc(3*mp.pi),mp.mpc(0),mp.mpc(0))
            p,_=pressure(state,force,4)
            self.assertEqual(p[(3,0,0)],-mp.j/2)
            self.assertEqual(p[(-3,0,0)],mp.j/2)
            complex_values=inverse({(0,0,0):mp.j},8)
            self.assertTrue(all(v==mp.j for v in complex_values))

    def test_layout_order_finiteness_and_derivative_admission(self) -> None:
        with self.assertRaises(ValueError):
            modes(12)
        for n,values,orders in ((12,{(0,0,0):mp.mpc(0)},(0,0,0)),
                                (8,{(4,0,0):mp.mpc(0)},(0,0,0)),
                                (8,{(0,0,0):mp.mpc(mp.inf)},(0,0,0)),
                                (8,{(0,0,0):mp.mpc(0)},(-1,0,0)),
                                (8,{(0,0,0):mp.mpc(0)},(2,1,0))):
            with self.assertRaises(ValueError):
                inverse(values,n,orders)
        state=taylor_green()
        force={k:(mp.mpc(0),mp.mpc(0),mp.mpc(0)) for k in modes(8)}
        with self.assertRaises(ValueError):
            pressure(dict(reversed(tuple(state.items()))),force,4)
        force[(0,0,0)]=(mp.mpc(mp.nan),mp.mpc(0),mp.mpc(0))
        with self.assertRaises(ValueError):
            pressure(state,force,4)

    def test_larger_admitted_direct_grids_preserve_scalar_means(self) -> None:
        with mp.workdps(80):
            for n in (16,24):
                wave=(n//2-1,1,0)
                values=inverse({(0,0,0):mp.mpc(3),wave:mp.mpc(1)},n)
                for point in ((0,0,0),(1,2,3),(3,1,2)):
                    expected=3+mp.exp(2*mp.pi*mp.j*sum(a*b for a,b in zip(wave,point))/n)
                    self.assertLess(abs(values[index(point,n)]-expected),mp.mpf('1e-75'))
