"""Closed polynomial identities and independently differentiated jet primitives."""
from dataclasses import FrozenInstanceError
from math import comb
import unittest
from mpmath import mp, mpf
from reference.jets import Jet, indices
from reference.dft import preflight, transform
from reference.steps import ho_phi
from reference.evaluator import evaluate
from reference.scalar import rational
from reference.tuples import triple, quadruple
from reference.verify_fields import scalar_force


class JetAlgebraTests(unittest.TestCase):
    def setUp(self) -> None:
        context = mp.workdps(80)
        context.__enter__()
        self.addCleanup(context.__exit__,None,None,None)

    def test_shape_and_dimension_refusals(self) -> None:
        for degree in (-1,5,100):
            with self.assertRaisesRegex(ValueError,'degrees'):
                indices(degree)
        with self.assertRaisesRegex(ValueError,'coefficient count'):
            Jet(4,(mp.mpf(0),))
        for axis,degree in ((4,4),(-1,4),(0,0)):
            with self.assertRaisesRegex(ValueError,'axis or degree'):
                Jet.variable(mp.mpf(1),axis,degree)
        x = Jet.variable(mp.mpf(1),0)
        with self.assertRaisesRegex(ValueError,'degree mismatch'):
            _ = x+Jet.constant(1,2)
        with self.assertRaisesRegex(ValueError,'positive constant'):
            _ = (-x)**rational('1/2')
        with self.assertRaisesRegex(ValueError,'derivative axis'):
            x.derivative(4)
        with self.assertRaises(ValueError):
            triple((1,2))
        with self.assertRaises(ValueError):
            quadruple((1,2,3,4,5))

    def test_exp_power_and_reciprocal_derivatives(self) -> None:
        x = Jet.variable(mp.mpf(2),0)
        for exponent in (mp.mpf(-2),rational('1/4'),mp.mpf(3)):
            power = x**exponent
            for order in range(5):
                def function(value: mpf) -> mpf:
                    return value**exponent
                self.assertLess(abs(power.partial((order,0,0,0))-mp.diff(function,mp.mpf(2),order)),mp.mpf('1e-70'))
        reciprocal = 1/x
        identity = reciprocal*x
        self.assertEqual(identity.value,1)
        self.assertLess(max(abs(c) for c in identity.coefficients[1:]),mp.mpf('1e-70'))
        for order in range(5):
            self.assertLess(abs(x.exp().partial((order,0,0,0))-mp.exp(2)),mp.mpf('1e-70'))
        self.assertEqual(len(indices(4)),70)
        constant = Jet.constant(2,0)
        self.assertEqual(constant.exp().value,mp.exp(2))

    def test_mixed_polynomial_factorials(self) -> None:
        x,y,z,t = (Jet.variable(mp.mpf(0),i) for i in range(4))
        polynomial = x*x*y + 3*z*t + 7
        self.assertEqual(polynomial.partial((2,1,0,0)),2)
        self.assertEqual(polynomial.partial((0,0,1,1)),3)
        self.assertEqual(polynomial.derivative(0).derivative(1).coefficients,
                         polynomial.derivative(1).derivative(0).coefficients)
        self.assertEqual(polynomial.value,7)

    def test_reference_work_refusals(self) -> None:
        with self.assertRaisesRegex(ValueError,'grid or payload'):
            transform([mp.mpc(0)],4)
        with self.assertRaisesRegex(ValueError,'80 or 120'):
            preflight(4,64,1024**3)
        with self.assertRaisesRegex(ArithmeticError,'exhausted'):
            ho_phi(mp.mpf(-1),1,max_terms=1)

    def test_force_gradient_independent_difference(self) -> None:
        point = tuple(rational(s) for s in ('1/32','1/64','1/10','1/1024'))
        evaluated = evaluate(*point)
        self.assertGreater(evaluated.root_iterations,0)
        # Vary a physical coordinate independently of the Taylor algebra.
        for axis in range(3):
            errors: list[mpf] = []
            for power in (5,6,7):
                h = mp.mpf(10)**(-power)
                left = list(point)
                right = list(point)
                left[axis] -= h
                right[axis] += h
                lo = scalar_force(quadruple(left))
                hi = scalar_force(quadruple(right))
                error = max(abs((a-b)/(2*h)-evaluated.force_gradient[c][axis])
                            for c,(a,b) in enumerate(zip(hi,lo)))
                errors.append(error)
            self.assertGreater(errors[0]/errors[1],90)
            self.assertGreater(errors[1]/errors[2],90)


class CompleteJetDomainTests(unittest.TestCase):
    def test_every_supported_degree(self) -> None:
        with mp.workdps(80):
            for degree in range(5):
                self.assertEqual(len(indices(degree)),comb(degree+4,4))
                x = Jet.constant(2,0) if degree == 0 else Jet.variable(mp.mpf(2),0,degree)
                power = x**rational('1/4')
                exponential = x.exp()
                for order in range(degree+1):
                    expected = mp.diff(lambda v:v**rational('1/4'),mp.mpf(2),order)
                    self.assertLess(abs(power.partial((order,0,0,0))-expected),mp.mpf('1e-70'))
                    self.assertLess(abs(exponential.partial((order,0,0,0))-mp.exp(2)),mp.mpf('1e-70'))
                if degree:
                    quotient = 3/x
                    self.assertEqual((quotient*x).value,3)
                    self.assertLess(abs(quotient.partial((1,0,0,0))+mp.mpf(3)/4),mp.mpf('1e-70'))

    def test_strict_shapes_nonfinite_values_and_freezing(self) -> None:
        with self.assertRaisesRegex(ValueError,'coefficient count'):
            Jet(4,(mp.mpf(0),)*71)
        for value in (mp.mpf(0),mp.mpf('-0.5')):
            with self.assertRaisesRegex(ValueError,'positive constant'):
                _ = Jet.constant(value)**rational('1/4')
        with self.assertRaisesRegex(ValueError,'degree mismatch'):
            _ = Jet.constant(1,2)+Jet.constant(1,4)
        for value in (mp.inf,mp.nan):
            with self.assertRaisesRegex(ValueError,'coefficients must be finite'):
                Jet.constant(value)
            with self.assertRaisesRegex(ValueError,'exponents must be finite'):
                _ = Jet.constant(1)**value
        frozen = Jet.constant(2)
        with self.assertRaises(FrozenInstanceError):
            setattr(frozen,'degree',3)

    def test_integer_substitutability(self) -> None:
        class Integer(int):
            pass
        x = Jet.variable(mp.mpf(2),Integer(1),Integer(4))
        self.assertEqual(x.partial((0,1,0,0)),1)
        self.assertEqual(x.derivative(Integer(1)).value,1)
        self.assertEqual((x+Jet.constant(1,4)).value,3)
