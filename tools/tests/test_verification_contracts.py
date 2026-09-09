"""Subprocess limits, failure details, and truthful scientific-drift attribution."""
import json
from pathlib import Path
import subprocess
import sys
from unittest.mock import patch
from tools.json_types import JsonObject, decode, object_value
from tools.tests.support import CheckoutTestCase
from tools.verify_design import execute, output_path, write_report


class VerificationContracts(CheckoutTestCase):
    def preserved(self) -> JsonObject:
        return dict(object_value(decode((self.root/'docs/design/navier-runtime-verification-results.json').read_text())))

    def test_subprocess_budget_and_success_metadata(self) -> None:
        calls: list[list[str]] = []
        def invoke(args: list[str], **options: object) -> subprocess.CompletedProcess[str]:
            calls.append(args)
            self.assertEqual(args,[sys.executable,'-E',str(self.root/'docs/design/navier-runtime-verification.py')])
            self.assertEqual(options['cwd'],self.root)
            self.assertEqual(options['timeout'],180)
            self.assertIs(options['capture_output'],True)
            self.assertEqual(options['encoding'],'utf-8')
            return subprocess.CompletedProcess(args,0,json.dumps(self.preserved()),'')
        with patch('tools.verify_design.subprocess.run',side_effect=invoke):
            result = execute(self.root)
        self.assertEqual(len(calls),1)
        metadata = object_value(result['bootstrap_execution'])
        self.assertIs(metadata['assertions_enabled'],True)
        self.assertIs(metadata['matches_preserved_report'],True)
        self.assertEqual(metadata['repository_checks'],'passed')

    def test_failure_tail_is_bounded_and_redacted(self) -> None:
        prefix = 'begin:'+str(self.root)+':'
        tail = prefix+'x'*(4000-len(prefix))
        completed = subprocess.CompletedProcess(['fixture'],7,'','older discarded material'+tail)
        with patch('tools.verify_design.subprocess.run',return_value=completed):
            result = execute(self.root)
        self.assertEqual(result['stage'],'mathematical_checks')
        self.assertEqual(result['returncode'],7)
        self.assertEqual(result['details'],tail.replace(str(self.root),'<repo>'))

    def test_scope_mismatch_and_exact_changed_keys(self) -> None:
        original = self.preserved()
        for unperformed in ([],['zzzz'],['AAAA']):
            altered = dict(original)
            altered['not_performed'] = unperformed
            completed = subprocess.CompletedProcess(['fixture'],0,json.dumps(altered),'')
            with patch('tools.verify_design.subprocess.run',return_value=completed):
                with self.assertRaisesRegex(ValueError,'unexpected status or evidence scope'):
                    execute(self.root)
        changed = dict(original)
        del changed['scales']
        changed['extra_channel'] = 1
        changed['coefficients'] = {'changed':True}
        completed = subprocess.CompletedProcess(['fixture'],0,json.dumps(changed),'')
        with patch('tools.verify_design.subprocess.run',return_value=completed):
            with self.assertRaises(ValueError) as error:
                execute(self.root)
        self.assertEqual(str(error.exception),"Recomputed checks differ from preserved evidence in: ['coefficients', 'extra_channel', 'scales']")

    def test_nested_report_creation_and_suffix_refusal(self) -> None:
        destination = output_path(self.root,Path('work/new/deeper/report.JSON'))
        write_report(destination,{'status':'failed','details':'preserved'})
        self.assertEqual(object_value(decode(destination.read_text())),{'status':'failed','details':'preserved'})
        for suffix in ('.csv','.txt',''):
            with self.assertRaisesRegex(ValueError,'.json suffix'):
                output_path(self.root,Path('work/report'+suffix))

    def test_documented_default_root_from_another_directory(self) -> None:
        for script in ('check_repository.py','verify_design.py'):
            completed = subprocess.run([sys.executable,str(self.root/'tools'/script)],cwd=self.root.parent,
                                       capture_output=True,text=True,timeout=180)
            self.assertEqual(completed.returncode,0,completed.stderr+completed.stdout)
            self.assertEqual(object_value(decode(completed.stdout))['status'],'passed')
