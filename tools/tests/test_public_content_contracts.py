"""File-type dispatch, complete link traversal, and public template boundaries."""
from tools.check_repository import check_links, check_public_content, unfenced_lines
from tools.tests.support import CheckoutTestCase


class PublicContentTests(CheckoutTestCase):
    def test_counts_and_markdown_only_link_scanning(self) -> None:
        root = self.root/'small'
        root.mkdir()
        (root/'one.md').write_text('[two](two.MD)')
        (root/'two.MD').write_text('[one](one.md)')
        (root/'data.json').write_text('{"description":"[not markdown](missing.txt)"}')
        report = check_public_content(root)
        self.assertEqual(report['public_files_scanned'],3)
        self.assertEqual(report['markdown_files_checked'],2)
        self.assertEqual(report['local_file_links_checked'],2)
        self.assertEqual(report['private_inclusion_screen'],'passed')
        self.assertEqual(check_links(root,root/'one.md','[one](one.md) [two](two.MD)'),2)

    def test_external_links_do_not_hide_later_local_links(self) -> None:
        text = '[mail](mailto:example@example.invalid) [web](//example.invalid/path) [local](README.md)'
        self.assertEqual(check_links(self.root,self.root/'README.md',text),1)
        with self.assertRaisesRegex(ValueError,'Missing local Markdown target'):
            check_links(self.root,self.root/'README.md','[web](https://example.invalid) [bad](missing.md)')

    def test_both_fence_kinds_must_close_with_their_own_kind(self) -> None:
        for opening,other in (('```','~~~'),('~~~','```')):
            text = f'before\n{opening}\n{other}\nhidden\n{opening}\nafter'
            self.assertEqual(unfenced_lines(text),'before\nafter')

    def test_env_example_is_public_but_other_env_files_are_not(self) -> None:
        root = self.root/'small'
        root.mkdir()
        (root/'.env.example').write_text('PUBLIC_EXAMPLE=1\n')
        self.assertEqual(check_public_content(root)['public_files_scanned'],1)
        for name in ('.env','.env.alpha','.env.zulu'):
            path = root/name
            path.write_text('TEST_PLACEHOLDER=1\n')
            with self.assertRaisesRegex(ValueError,'Private material'):
                check_public_content(root)
            path.unlink()
