"""Independent scalar, implicit residual and closed-form tests for early P00B work."""
import unittest
from mpmath import mp, mpf
from reference.scalar import CoordinateUnresolved, fields, rational as r, root, step
from reference.evaluator import evaluate, implicit_root, smooth_step
from reference.jets import Jet


class ReferenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.context = mp.workdps(80)
        self.context.__enter__()
        self.addCleanup(self.context.__exit__, None, None, None)

    def test_admissible_root_and_residual(self) -> None:
        for z in ('0', '1/10', '-2/5'):
            for t in ('0', '1/512', '127/16384'):
                result = root(r(z),r(t))
                self.assertLessEqual(result.lower,result.value)
                self.assertLessEqual(result.value,result.upper)
                self.assertLess(abs(result.residual), mp.mpf('1e-78'))
                eta = r(z)*result.value**r('-3/8')
                self.assertLess(abs(eta),1)

    def test_root_refusals(self) -> None:
        with self.assertRaises(CoordinateUnresolved):
            root(r('1/10'),r('1/512'),1)
        for t in ('-1', '1/128', 'nan'):
            with self.assertRaises(ValueError):
                root(mp.mpf(0),mp.mpf(t))
        with self.assertRaises(ValueError):
            root(mp.mpf(0),mp.mpf(0),0)

    def test_axis_closed_form_and_startup(self) -> None:
        for t in ('0', '1/512', '1/256'):
            value = evaluate(mp.mpf(0),mp.mpf(0),mp.mpf(0),r(t))
            expected = step(512*r(t))*r('1/32')*(r('1/128')-r(t))**r('-5/8')
            self.assertLess(abs(value.velocity[2]-expected),mp.mpf('1e-76'))
            self.assertEqual(value.velocity[:2],(0,0))
            if t == '0':
                self.assertEqual(value.force,(0,0,0))

    def test_scalar_field_agreement_and_divergence(self) -> None:
        for point in (('1/32','-1/64','1/10','1/1024'),
                      ('7/20','0','1/20','1/256'), ('21/50','0','0','1/256')):
            xyz = tuple(r(s) for s in point)
            value = evaluate(*xyz)
            scalar, pressure = fields(*xyz)
            self.assertLess(max(abs(a-b) for a,b in zip(value.velocity,scalar)),mp.mpf('1e-70'))
            self.assertLess(abs(value.pressure_raw-pressure),mp.mpf('1e-70'))
            self.assertLess(abs(value.divergence),mp.mpf('1e-70'))
            self.assertLess(max(abs(c) for c in value.root_residual_coefficients),mp.mpf('1e-65'))

    def test_implicit_derivatives_against_scalar_differentiation(self) -> None:
        z,t = r('1/10'),r('1/1024')
        solution = implicit_root(Jet.variable(z,2),Jet.variable(t,3))
        q, residual = solution.jet, solution.residual
        def scalar_root(a: mpf, b: mpf) -> mpf:
            return root(a,b).value
        for dz,dt in ((1,0),(0,1),(2,0),(1,1),(0,2),(2,1)):
            oracle = mp.diff(scalar_root,(z,t),(dz,dt))
            self.assertLess(abs(q.partial((0,0,dz,dt))-oracle),mp.mpf('1e-65'))
        self.assertLess(max(abs(c) for c in residual.coefficients),mp.mpf('1e-65'))

    def test_scalar_difference_refinement(self) -> None:
        z,t = r('1/10'),r('1/1024')
        q = implicit_root(Jet.variable(z,2),Jet.variable(t,3)).jet
        expected = q.partial((0,0,1,0))
        errors: list[mpf] = []
        for power in (3,4,5,6):
            h = mp.mpf(10)**(-power)
            numerical = (root(z+h,t).value-root(z-h,t).value)/(2*h)
            errors.append(abs(numerical-expected))
        for coarse,fine in zip(errors,errors[1:]):
            self.assertGreater(coarse/fine,90)
            self.assertLess(coarse/fine,110)

    def test_nonfinite_space_refused(self) -> None:
        for evaluator in (fields,evaluate):
            with self.assertRaises(ValueError):
                evaluator(mp.inf,mp.mpf(0),mp.mpf(0),mp.mpf(0))

    def test_cutoff_derivatives_and_flats(self) -> None:
        for s in ('-1','0','1/4','1/2','3/4','1','2'):
            value = r(s)
            jet = smooth_step(Jet.variable(value,0))
            for degree in range(5):
                expected = mp.diff(step,value,degree)
                self.assertLess(abs(jet.partial((degree,0,0,0))-expected),mp.mpf('1e-65'))
