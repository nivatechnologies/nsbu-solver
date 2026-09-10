"""Independent scalar, geometry, gauge and bounded-failure controls for periodic pressure."""
from fractions import Fraction
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.pressure_gauge import MeanPlan, evaluate, radial_pressure
from reference.quadrature import simpson
from reference.scalar import CoordinateUnresolved, fields, rational, root


class PressureGaugeTests(unittest.TestCase):
    def test_simpson_exact_polynomials_complete_nodes_and_fourth_order_error(self) -> None:
        with mp.workdps(80):
            visited: list[mpf]=[]
            def cubic(x: mpf) -> mpf:
                visited.append(x)
                return x**3+2*x+1
            self.assertLess(abs(simpson(cubic,mp.mpf(-1),mp.mpf(2),8)-rational('39/4')),mp.mpf('1e-75'))
            self.assertEqual(len(visited),9)
            self.assertEqual((visited[0],visited[-1]),(mp.mpf(-1),mp.mpf(2)))
            errors=[abs(simpson(lambda x:x**4,mp.mpf(0),mp.mpf(1),n)-rational('1/5')) for n in (4,8,16)]
            self.assertLess(abs(errors[0]/errors[1]-16),mp.mpf('1e-70'))
            self.assertLess(abs(errors[1]/errors[2]-16),mp.mpf('1e-70'))
            self.assertEqual(simpson(cubic,mp.mpf(1),mp.mpf(1),2),0)

    def test_cylindrical_jacobian_integrates_the_whole_ball(self) -> None:
        with mp.workdps(80):
            radius=rational('21/50')
            def cross_section(z: mpf) -> mpf:
                high=max(mp.mpf(0),radius*radius-z*z)
                return simpson(lambda _w:mp.mpf(1),mp.mpf(0),high,8)
            volume=mp.pi*simpson(cross_section,-radius,radius,8)
            self.assertLess(abs(volume-4*mp.pi*radius**3/3),mp.mpf('1e-75'))

    def test_pressure_formula_and_full_mean_match_separate_cartesian_scalar(self) -> None:
        with mp.workdps(80):
            t=rational('1/1024')
            for x,z in (('0','0'),('1/8','-1/9'),('1/3','1/10'),('2/5','1/5')):
                xv,zv=rational(x),rational(z)
                actual=radial_pressure(xv*xv,zv,t,root(zv,t).value)
                self.assertLess(abs(actual-fields(xv,mp.mpf(0),zv,t)[1]),mp.mpf('1e-75'))

    def test_exact_core_matches_independent_high_resolution_scalar_quadrature(self) -> None:
        with mp.workdps(80):
            t=rational('1/1024')
            # Independently sample the Cartesian scalar formula over the full radial
            # interval. Its successive refinements converge to the core/collar split.
            from reference.pressure_gauge import slab
            z=mp.mpf(0)
            high=rational('441/2500')
            values=[simpson(lambda w:fields(mp.sqrt(w),mp.mpf(0),z,t)[1],mp.mpf(0),high,n)
                    for n in (128,256,512)]
            exact_core=slab(z,t,MeanPlan(Fraction(1,1024),8,512))
            errors=[abs(v-exact_core) for v in values]
            self.assertGreater(errors[0]/errors[1],15)
            self.assertGreater(errors[1]/errors[2],15)
            self.assertLess(errors[2]/abs(exact_core),mp.mpf('1e-7'))

    def test_global_gauge_preserves_differences_and_extends_outside_raw_support(self) -> None:
        plan=MeanPlan(Fraction(1,256),16,16,120)
        before=mp.dps
        mean=evaluate(plan)
        self.assertEqual(mp.dps,before)
        self.assertLess(mean.value,0)
        with mp.workdps(120):
            t=rational('1/256')
            origin=fields(mp.mpf(0),mp.mpf(0),mp.mpf(0),t)[1]
            outside=fields(rational('49/100'),mp.mpf(0),mp.mpf(0),t)[1]
            self.assertEqual(outside,0)
            self.assertEqual(mean.subtract(outside),-mean.value)
            self.assertLess(abs((mean.subtract(origin)-mean.subtract(outside))-origin),mp.mpf('1e-110'))
        self.assertEqual(mp.dps,before)
        self.assertEqual(evaluate(MeanPlan(Fraction(0),2,2)).value,0)
        with self.assertRaises(ValueError):mean.subtract(mp.inf)

    def test_admission_bounds_actual_work_and_refuses_before_root_evaluation(self) -> None:
        plan=MeanPlan(Fraction(1,256),8,16)
        self.assertEqual(plan.evaluations,153)
        self.assertEqual(plan.preflight()['maximum_root_iterations'],9*2048)
        self.assertEqual(plan.preflight()['accepted_pde_windows'],0)
        for t in (Fraction(-1),Fraction(1,128)):
            with self.assertRaises(ValueError):MeanPlan(t,8,8)
        for n in (0,3,1026):
            with self.assertRaises(ValueError):MeanPlan(Fraction(0),n,8)
            with self.assertRaises(ValueError):MeanPlan(Fraction(0),8,n)
        for dps in (0,81,121):
            with self.assertRaises(ValueError):MeanPlan(Fraction(0),8,8,dps)
        for cap in (0,2049):
            with self.assertRaises(ValueError):MeanPlan(Fraction(0),8,8,80,cap)
        with patch('reference.pressure_gauge.root',side_effect=AssertionError('Must not evaluate')):
            self.assertEqual(plan.preflight()['collar_pressure_evaluations'],153)
            with self.assertRaises(ValueError):MeanPlan(Fraction(0),8,8,cap_bytes=1)
        before=mp.dps
        with self.assertRaises(CoordinateUnresolved):evaluate(MeanPlan(Fraction(1,256),8,8,80,1))
        self.assertEqual(mp.dps,before)

    def test_quadrature_rejects_bad_domains_and_nonfinite_samples(self) -> None:
        for low,high,n in ((mp.mpf(0),mp.mpf(1),1),(mp.mpf(0),mp.mpf(1),3),
                           (mp.inf,mp.mpf(1),2),(mp.mpf(0),mp.inf,2),(mp.mpf(1),mp.mpf(0),2)):
            with self.assertRaises(ValueError):simpson(lambda x:x,low,high,n)
        with self.assertRaises(ArithmeticError):simpson(lambda _x:mp.nan,mp.mpf(0),mp.mpf(1),2)
