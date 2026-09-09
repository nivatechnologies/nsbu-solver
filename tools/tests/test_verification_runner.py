"""Execution, report-write and CLI refusal contracts independent of SymPy runtime."""
from contextlib import redirect_stdout
from io import StringIO
import json
from pathlib import Path
import runpy
import subprocess
import sys
from unittest.mock import patch
import tools.verify_design as runner
import tools.check_repository as check_repository
from tools.json_types import decode, object_value
from tools.tests.support import CheckoutTestCase


class RunnerTests(CheckoutTestCase):
    def scientific_output(self) -> str:
        return (self.root/'docs/design/navier-runtime-verification-results.json').read_text()

    def process(self, code: int = 0, text: str | None = None) -> subprocess.CompletedProcess[str]:
        return subprocess.CompletedProcess(['reference'],code,
                                           self.scientific_output() if text is None else text,'failure')

    def test_execution_success_and_repository_refusal(self) -> None:
        with patch('tools.verify_design.subprocess.run',return_value=self.process()):
            report = runner.execute(self.root)
        self.assertEqual(report['status'],'passed')
        metadata = object_value(report['bootstrap_execution'])
        self.assertTrue(metadata['matches_preserved_report'])
        self.assertEqual(metadata['dependencies'],runner.DEPENDENCIES)
        (self.root/'NOTICE').unlink()
        self.assertEqual(runner.execute(self.root)['stage'],'repository')

    def test_dependency_and_process_errors(self) -> None:
        with patch('tools.verify_design.importlib.metadata.version',return_value='wrong'):
            with self.assertRaisesRegex(ValueError,'requirements-dev'):
                runner.execute(self.root)
        with patch('tools.verify_design.subprocess.run',return_value=self.process(2)):
            report = runner.execute(self.root)
        self.assertEqual(report['stage'],'mathematical_checks')
        self.assertEqual(report['returncode'],2)
        for text in ('{}','{"status":"failed"}',self.scientific_output().replace('"passed"','"rejected"')):
            with patch('tools.verify_design.subprocess.run',return_value=self.process(text=text)):
                with self.assertRaisesRegex(ValueError,'unexpected status'):
                    runner.execute(self.root)

    def test_scientific_drift_refused(self) -> None:
        original = self.scientific_output()
        drift = dict(object_value(decode(original)))
        drift['unexpected_channel'] = 'changed'
        with patch('tools.verify_design.subprocess.run',return_value=self.process(text=json.dumps(drift))):
            with self.assertRaisesRegex(ValueError,'differ from preserved'):
                runner.execute(self.root)

    def test_report_paths_and_atomic_failure(self) -> None:
        self.assertEqual(runner.output_path(self.root,Path('work/out.json')),self.root/'work/out.json')
        outside = Path(self.temporary.name)/'outside.json'
        self.assertEqual(runner.output_path(self.root,outside),outside)
        for path in ('.','README.md','work/out.txt'):
            with self.assertRaises(ValueError):
                runner.output_path(self.root,Path(path))
        output = self.root/'work/report.json'
        runner.write_report(output,{'status':'passed'})
        self.assertEqual(object_value(decode(output.read_text()))['status'],'passed')
        with patch('tools.verify_design.os.replace',side_effect=OSError('rename refused')):
            with self.assertRaisesRegex(OSError,'rename refused'):
                runner.write_report(output,{'status':'failed'})
        self.assertEqual(list(output.parent.glob('*.tmp')),[])
        self.assertEqual(object_value(decode(output.read_text()))['status'],'passed')
        with patch('tools.verify_design.tempfile.NamedTemporaryFile',side_effect=OSError('open refused')):
            with self.assertRaisesRegex(OSError,'open refused'):
                runner.write_report(output,{'status':'failed'})

    def invoke(self, arguments: list[str]) -> tuple[int,str]:
        output = StringIO()
        with patch.object(sys,'argv',['verify_design', '--root', str(self.root),*arguments]),redirect_stdout(output):
            code = runner.main()
        return code,output.getvalue()

    def test_cli_success_and_refusals(self) -> None:
        with patch('tools.verify_design.execute',return_value={'status':'passed'}):
            code,text = self.invoke(['--output','work/result.json'])
        self.assertEqual(code,0)
        self.assertEqual(object_value(decode(text))['status'],'passed')
        self.assertEqual(self.invoke(['--output','LICENSE'])[0],2)
        with patch('tools.verify_design.execute',side_effect=ValueError('invalid')):
            code,text = self.invoke([])
        self.assertEqual(code,1)
        self.assertEqual(object_value(decode(text))['stage'],'verification')
        with patch('tools.verify_design.execute',return_value={'status':'passed'}),patch('tools.verify_design.write_report',side_effect=OSError('disk full')):
            code,text = self.invoke([])
        self.assertEqual(code,1)
        self.assertEqual(object_value(decode(text))['stage'],'report_write')

    def test_optimized_cli_and_module_entries(self) -> None:
        class Flags:
            optimize = 1
        with patch('tools.verify_design.sys.flags',Flags()):
            code,text = self.invoke([])
        self.assertEqual(code,1)
        self.assertIn('Optimized Python',text)
        # Execute the real module entry points against an intentionally incomplete root.
        (self.root/'NOTICE').unlink()
        for module in ('tools.check_repository','tools.verify_design'):
            with patch.object(sys,'argv',[module,'--root',str(self.root)]),redirect_stdout(StringIO()):
                with self.assertRaises(SystemExit):
                    runpy.run_path(str(Path(__file__).resolve().parents[1]/(module.rsplit('.',1)[1]+'.py')),run_name='__main__')
        with patch.object(sys,'argv',['check_repository','--root',str(self.root)]),redirect_stdout(StringIO()):
            self.assertEqual(check_repository.main(),1)
