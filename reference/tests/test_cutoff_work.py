"""The logistic cutoff never requests an exponentially growing intermediate."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.evaluator import smooth_step
from reference.jets import Jet
from reference.scalar import step


class CutoffWorkTests(unittest.TestCase):
    def test_exponential_arguments_are_nonpositive(self) -> None:
        with mp.workdps(80):
            original = mp.exp
            def bounded_exp(value: mpf | int) -> mpf:
                self.assertLessEqual(value,0)
                return original(value)
            for text in ('1/10000000000','49/100','1/2','51/100','9999999999/10000000000'):
                value = mp.mpf(text)
                expected = step(value)
                with patch.object(mp,'exp',side_effect=bounded_exp):
                    self.assertEqual(step(value),expected)
                    self.assertLess(abs(smooth_step(Jet.variable(value,0)).value-expected),mp.mpf('1e-75'))
