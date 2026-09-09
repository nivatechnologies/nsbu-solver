"""Reporting uses the declared comparison precision and exact rejection threshold."""
import unittest
from unittest.mock import patch
from mpmath import mp, mpf
from reference.dft import Spectrum
from reference.tests.test_field_contracts import force_value
from reference.verify_fields import FieldSample, compare as field_compare
from reference.verify_steps import Study, compare as step_compare


class ReportExecutionTests(unittest.TestCase):
    def test_comparison_precision_and_nonzero_equal_states(self) -> None:
        with mp.workdps(120):
            original = mp.nstr
            def rendered(value: mpf, digits: int) -> str:
                self.assertEqual(mp.dps,120)
                return original(value,digits)
            field = FieldSample('same',('0','0','0','0'),force_value(mp.mpf(7)),mp.mpf(0))
            spectrum: Spectrum = {(0,0,0):(mp.mpc(7),mp.mpc(0),mp.mpc(0))}
            item = Study(80,'CM',{},spectrum,spectrum,mp.mpf(0))
            with patch.object(mp,'nstr',side_effect=rendered):
                self.assertEqual(field_compare(field,field)['force_scaled_change'],'0.0')
                self.assertEqual(step_compare(item,item)[0]['max_full_band_component_change'],'0.0')
            zero: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            boundary: Spectrum = {(0,0,0):(mp.mpc(mp.mpf('1e-60')),mp.mpc(0),mp.mpc(0))}
            low,high = Study(80,'CM',{},zero,zero,mp.mpf(0)),Study(120,'CM',{},boundary,boundary,mp.mpf(0))
            with self.assertRaisesRegex(ArithmeticError,'DFT step arithmetic'):
                step_compare(low,high)
