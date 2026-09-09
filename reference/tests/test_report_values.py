"""Nonzero report values preserve precision, conditioning and full-band errors."""
import unittest
from mpmath import mp
from reference.dft import Spectrum
from reference.evaluator import Evaluation
from reference.verify_fields import FieldSample, compare as field_change, sample_report
from reference.verify_steps import Study, compare as step_change, study_report


class ReportValueTests(unittest.TestCase):
    def test_field_decimal_precision_and_conditioning(self) -> None:
        with mp.workdps(80):
            delta = mp.mpf('0.123456789123456789123456789')
            vector = (mp.mpf(7),mp.mpf(-7),mp.mpf(0))
            gradient = ((mp.mpf(1)/3,mp.mpf(-2)/3,delta),)*3
            value = Evaluation(vector,mp.mpf(0),vector,delta,(-delta,delta/2),
                               ((mp.mpf(-1),mp.mpf(0),mp.mpf(0),mp.mpf(0)),)*3,9,gradient)
            item = FieldSample('nonzero',('1','2','3','4'),value,mp.mpf('1.234567890123456789e-80'))
            report = sample_report(item,6)
            self.assertEqual(report,{'sample':'nonzero','coordinates':('1','2','3','4'),
                'velocity':['7.0','-7.0','0.0'],'force':['7.0','-7.0','0.0'],
                'force_oracle_scaled_error':'1.23456789012e-80','divergence':'0.123456789123',
                'root_iterations':9,'force_gradient':[['0.333333','-0.666667','0.123457']]*3,
                'max_root_jet_residual':'0.123456789123',
                'assembly_conditioning':['0.142857142857','0.142857142857','1.0e+60']})

    def test_full_step_report_preserves_complex_values(self) -> None:
        with mp.workdps(80):
            zero = mp.mpc(0)
            full: Spectrum = {(1,0,0):(mp.mpc(mp.mpf(1)/3,mp.mpf(-2)/3),zero,zero)}
            fine: Spectrum = {(1,0,0):(zero,zero,mp.mpc(7))}
            item = Study(6,'HO',{'reserved':123},full,fine,mp.mpf('0.123456789123456789123456789'))
            self.assertEqual(study_report(item),{'precision':6,'method':'HO','preflight':{'reserved':123},
                'full_step':[{'mode':(1,0,0),'value':[['0.333333','-0.666667'],['0.0','0.0'],['0.0','0.0']]}],
                'two_half_steps':[{'mode':(1,0,0),'value':[['0.0','0.0'],['0.0','0.0'],['7.0','0.0']]}],
                'raw_local_discrepancy':'0.12345678912345678912'})

    def test_nonzero_arithmetic_differences(self) -> None:
        with mp.workdps(120):
            delta = mp.mpf('1.234567890123456789e-70')
            zero = mp.mpf(0)
            empty = (zero,zero,zero)
            def evaluated(first: str) -> Evaluation:
                return Evaluation(empty,zero,(mp.mpf(first),zero,zero),zero,(zero,),
                                  ((zero,zero,zero,zero),)*3,0,(empty,)*3)
            low = FieldSample('precision',('0','0','0','0'),evaluated('0'),zero)
            high = FieldSample('precision',low.coordinates,evaluated(str(delta)),zero)
            self.assertEqual(field_change(low,high),{'sample':'precision','force_scaled_change':'1.23456789012e-70'})
            left: Spectrum = {(0,0,0):(mp.mpc(0),mp.mpc(0),mp.mpc(0))}
            right: Spectrum = {(0,0,0):(mp.mpc(delta),mp.mpc(0),mp.mpc(0))}
            studies = Study(80,'CM',{},left,right,zero),Study(120,'CM',{},right,left,zero)
            self.assertEqual(step_change(*studies),[
                {'method':'CM','path':path,'max_full_band_component_change':'1.23456789012e-70'}
                for path in ('full_step','two_half_steps')])
