import importlib.util
from pathlib import Path
import json
import os
import subprocess
import tempfile
import unittest
from datetime import date, timedelta
from unittest.mock import patch


def load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


calendar = load('calendar-sync')
projects = load('project-status')


class IntegrationTests(unittest.TestCase):
    def test_structured_video_link_is_preferred(self):
        event = {'conferenceData': {'entryPoints': [
            {'entryPointType': 'phone', 'uri': 'tel:+123'},
            {'entryPointType': 'video', 'uri': 'https://meet.google.com/test'}]},
            'description': 'https://example.com/not-the-call'}
        self.assertEqual(calendar.meeting_link(event), ('https://meet.google.com/test', 'Meet'))

    def test_description_links_are_decoded_and_provider_checked(self):
        event = {'description': '<a href="https://us02web.zoom.us/j/123?x=1&amp;y=2">Join</a>'}
        self.assertEqual(calendar.meeting_link(event), ('https://us02web.zoom.us/j/123?x=1&y=2', 'Zoom'))
        for value in ['https://zoom.us.evil.example/j/123', 'https://user:secret@zoom.us/j/123', 'javascript:alert(1)']:
            self.assertEqual(calendar.meeting_link({'description': value}), (None, None))

    def test_cache_covers_cross_month_weeks(self):
        for year in [2024, 2026, 2027]:
            day = date(year, 1, 1)
            while day.year == year:
                start, end = calendar.calendar_window(day.strftime('%Y-%m'))
                monday = day - timedelta(days=day.weekday())
                self.assertLessEqual(start, monday)
                self.assertLess(monday + timedelta(days=6), end)
                day += timedelta(days=1)

    def test_git_rename_paths_do_not_create_fake_entries(self):
        raw = (b'# branch.oid abc123456789\0# branch.head feature/my-work\0'
               b'# branch.upstream origin/main\0# branch.ab +2 -3\0'
               b'2 R. N... 100644 100644 100644 a b R100 new name\0? old name\0'
               b'? actual new file\0u UU details\0')
        state = projects.parse_status(raw)
        self.assertEqual((state['branch'], state['ahead'], state['behind']), ('feature/my-work', 2, 3))
        self.assertEqual((state['changed'], state['untracked'], state['conflicts']), (2, 1, 1))

    def test_detached_head_uses_commit(self):
        state = projects.parse_status(b'# branch.oid abc123456789\0# branch.head (detached)\0')
        self.assertEqual(state['branch'], 'abc12345 · detached')


class ProjectPresentationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.repo = self.directory / 'Voice-Agent'
        self.repo.mkdir()
        self.git('init', '-q', '-b', 'main')
        (self.repo / 'package.json').write_text(json.dumps({'name': 'vite_react_shadcn_ts'}))
        self.git('add', 'package.json')
        self.git('-c', 'user.name=Project Tests', '-c', 'user.email=projects@example.invalid',
                 '-c', 'commit.gpgsign=false', 'commit', '-qm', 'Initial test repository')

    def git(self, *args):
        return subprocess.run(['git', '-C', str(self.repo), *args], check=True,
                              capture_output=True).stdout

    def worktree(self, name='history-callbacks-design-6f7515'):
        checkout = self.directory / '.claude' / 'worktrees' / name
        self.git('worktree', 'add', '-qb', 'claude/' + name, str(checkout))
        return checkout

    def test_linked_worktree_uses_repository_title_and_identity(self):
        checkout = self.worktree()
        primary = projects.repository(self.repo, [])
        linked = projects.repository(checkout, [])
        self.assertEqual(primary.get('title'), 'Voice Agent')
        self.assertEqual(linked.get('title'), 'Voice Agent')
        self.assertEqual(primary.get('repository_id'), str(self.repo / '.git'))
        self.assertEqual(linked.get('repository_id'), str(self.repo / '.git'))
        self.assertEqual(linked['path'], str(checkout))

    def test_explicit_product_title_comes_from_primary_checkout(self):
        checkout = self.worktree()
        (self.repo / 'package.json').write_text(json.dumps({'productName': 'Voice Studio'}))
        (checkout / 'package.json').write_text(json.dumps({'productName': 'Generated Checkout'}))
        self.assertEqual(projects.repository(checkout, []).get('title'), 'Voice Studio')

    def test_separate_git_directory_uses_the_configured_primary_checkout(self):
        git_directory = self.directory / 'shared-git-metadata'
        self.git('init', '-q', '--separate-git-dir', str(git_directory))
        self.git('config', 'core.worktree', str(self.repo))
        checkout = self.worktree()
        linked = projects.repository(checkout, [])
        self.assertEqual(linked.get('title'), 'Voice Agent')
        self.assertEqual(linked.get('repository_id'), str(git_directory))

    def test_path_or_unknown_manifest_titles_do_not_become_widget_titles(self):
        for title in ['/tmp/private/project-a1b2c3d4', '../checkout', 'Unknown', '', 'Bad\nTitle']:
            with self.subTest(title=title):
                (self.repo / 'package.json').write_text(json.dumps({'displayName': title}))
                self.assertEqual(projects.repository(self.repo, []).get('title'), 'Voice Agent')

    def test_repository_scan_preserves_ports_changes_and_index(self):
        checkout = self.worktree()
        (checkout / 'new file').write_text('Untracked test content')
        index = Path(os.fsdecode(subprocess.run(
            ['git', '-C', str(checkout), 'rev-parse', '--absolute-git-dir'],
            check=True, capture_output=True).stdout).strip()) / 'index'
        before = (index.read_bytes(), index.stat().st_mtime_ns)
        sockets = [{'cwd': checkout, 'port': 5173, 'process': 'node'},
                   {'cwd': checkout / 'src', 'port': 9229, 'process': 'node'},
                   {'cwd': self.directory / 'other', 'port': 3000, 'process': 'node'}]
        result = projects.repository(checkout, sockets)
        self.assertEqual(result['untracked'], 1)
        self.assertEqual([(s['port'], s['web']) for s in result['servers']], [(5173, True), (9229, False)])
        self.assertEqual((index.read_bytes(), index.stat().st_mtime_ns), before)

    def test_scan_output_keeps_same_title_worktrees_as_one_repository(self):
        checkout = self.worktree()
        config = self.directory / 'config.json'
        config.write_text(json.dumps({'project_roots': [str(self.repo)]}))
        with patch.dict(os.environ, {'DESKTOP_CARDS_CONFIG': str(config)}):
            result = projects.scan()
        self.assertEqual({item['path'] for item in result['projects']}, {str(self.repo), str(checkout)})
        self.assertEqual({item.get('title') for item in result['projects']}, {'Voice Agent'})
        self.assertEqual({item.get('repository_id') for item in result['projects']}, {str(self.repo / '.git')})


if __name__ == '__main__':
    unittest.main()
