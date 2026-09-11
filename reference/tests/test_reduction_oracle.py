"""Independent closed-form tensor controls and strict output provenance before arithmetic."""
import hashlib
from pathlib import Path
import struct
import unittest
from mpmath import mp
from reference.reduction_audit import input, output
from reference.reduction_audit.oracle import evaluate
from reference.reduction_audit.study import errors

ROOT=Path(__file__).resolve().parents[2]


def actual() -> tuple[input.SampleInput,output.Magnitudes]:
    source=input.parse((ROOT/'crates/nsbu-benchmarks/data/reduction-n4.bin').read_bytes())
    result=output.parse((ROOT/'fixtures/reference/reduction-n4-magnitudes.bin').read_bytes(),source)
    return source,result


def scalar(value: float) -> bytes:
    return struct.pack('<d',value)


def synthetic() -> tuple[input.SampleInput,output.Magnitudes]:
    source,_=actual();data=bytearray(source.data);data[input.HEADER:]=bytes(len(data)-input.HEADER)
    data[input.HEADER:input.HEADER+8]=scalar(3)
    offset=input.HEADER+8*13*512;data[offset:offset+8]=scalar(4)
    source=input.parse(bytes(data))
    raw=bytearray(b'NSBUMAG1'+(4).to_bytes(8,'little')+(512).to_bytes(8,'little'))
    raw.extend(hashlib.sha256(data).digest());raw.extend(data[24:72]);raw.extend(bytes(384+12*512*8))
    raw[output.HEADER:output.HEADER+8]=scalar(5)
    return source,output.parse(bytes(raw),source)


class ReductionOutputTests(unittest.TestCase):
    def test_complete_input_digest_shape_floors_and_nonnegative_finite_words(self) -> None:
        source,result=actual()
        for data in (b'',result.data[:-1],result.data+b'x'):
            with self.assertRaises(ValueError):output.parse(data,source)
        for offset,word in ((0,0),(8,12),(16,1),(24,0),(56,0),(104,0x7ff0000000000000),(488,0xbff0000000000000)):
            data=bytearray(result.data);data[offset:offset+8]=word.to_bytes(8,'little')
            with self.assertRaises(ValueError):output.parse(bytes(data),source)
        for indices in ((6,0,0),(0,2,0),(0,0,512),(-1,0,0)):
            with self.assertRaises(ValueError):result.word(*indices)
        for indices in ((6,0),(0,2),(-1,0)):
            with self.assertRaises(ValueError):result.statistics(*indices)
        changed=bytearray(result.data);changed[488:496]=scalar(-0.0)
        self.assertEqual(output.parse(bytes(changed),source).word(0,0,0),1<<63)
        self.assertEqual(len(result.statistics(5,1)),4)

    def test_three_four_five_full_tensor_normalization_and_exact_zero_are_independent(self) -> None:
        source,result=synthetic()
        before=(source.data,result.data)
        for precision in (80,120):
            values=evaluate(source,result,precision)
            with mp.workdps(precision):
                self.assertEqual(values[0].exact_components[0],mp.sqrt(mp.mpf(25)/512))
                self.assertEqual(values[0].exact_components[1],5)
                self.assertEqual(values[0].rounded_magnitudes[0],mp.sqrt(mp.mpf(25)/512))
                self.assertEqual(values[0].magnitude_maximum_absolute_errors,(0,0))
                for group in values[1:]:self.assertEqual(group.exact_components,(0,0,0,0))
        self.assertEqual(before,(source.data,result.data))
        with self.assertRaises(ValueError):evaluate(source,result,79)

    def test_an_altered_finite_magnitude_is_measured_not_silently_replaced_by_exact_data(self) -> None:
        source,result=synthetic();data=bytearray(result.data);data[488:496]=scalar(4)
        changed=output.parse(bytes(data),source);value=evaluate(source,changed,80)[0]
        self.assertEqual(value.exact_components[1],5);self.assertEqual(value.rounded_magnitudes[1],4)
        self.assertEqual(value.magnitude_maximum_absolute_errors,(1,0))
        with mp.workdps(80):
            self.assertLess(abs(value.magnitude_rms_errors[0]-1/mp.sqrt(512)),mp.mpf('1e-79'))

    def test_precision_policy_keeps_zero_denominators_and_unresolved_arithmetic_visible(self) -> None:
        with mp.workdps(120):
            z=mp.mpf(0);one=mp.mpf(1)
            result=errors((0,0,0,0),(z,one,z,one),(z,z,one,one))
        first=result['rms_error'];self.assertIsInstance(first,dict)
        from tools.json_types import object_value
        self.assertEqual(object_value(first)['precision_to_error_ratio'],None)
        self.assertTrue(object_value(first)['precision_separated'])
        self.assertFalse(object_value(result['sampled_peak_error'])['precision_separated'])
        self.assertFalse(object_value(result['sampled_peak_relative_error'])['precision_separated'])
