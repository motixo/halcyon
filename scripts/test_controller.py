import fcntl
from concurrent.futures import ThreadPoolExecutor
from contextlib import redirect_stdout
import importlib.machinery
import importlib.util
import io
import json
import re
from pathlib import Path
import subprocess
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

loader = importlib.machinery.SourceFileLoader('controller', str(Path(__file__).with_name('desktop-cards')))
spec = importlib.util.spec_from_loader(loader.name, loader)
controller = importlib.util.module_from_spec(spec)
loader.exec_module(controller)


class ControllerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.config = Path(self.directory.name) / 'config.json'
        self.patch = patch.object(controller, 'CONFIG_FILE', self.config)
        self.patch.start()
        self.addCleanup(self.patch.stop)
        folder = patch.object(controller, 'CONFIG', Path(self.directory.name))
        folder.start()
        self.addCleanup(folder.stop)
        for attribute, name in [('BACKGROUND', 'background.ron'), ('CUSTOM', 'custom.ron'),
                                ('WALLPAPERS', 'wallpapers')]:
            isolated = patch.object(controller, attribute, Path(self.directory.name) / name)
            isolated.start()
            self.addCleanup(isolated.stop)
        # Never reach the installed helper: every subprocess is recorded.
        helper = patch.object(controller.subprocess, 'run',
                              return_value=subprocess.CompletedProcess([], 0, '', ''))
        self.run = helper.start()
        self.addCleanup(helper.stop)
        self.helper = str(controller.USER_DIR / '.local/bin/cosmic-desktop-cards')

    def links_calls(self):
        return [call.args[0] for call in self.run.call_args_list
                if call.args and call.args[0][1:2] == ['--apply-palette-links']]

    def assert_one_link_call(self, name, *flags):
        self.assertEqual(self.links_calls(), [[self.helper, '--apply-palette-links', name, *flags]])
        self.assertEqual(len(self.run.call_args_list), 1, 'the helper is the only command')

    def test_settings_launches_the_native_window_on_the_requested_page(self):
        with patch.object(controller.subprocess, 'run') as run:
            controller.settings('calendar')
            run.assert_called_once_with(
                [str(controller.USER_DIR / '.local/bin/cosmic-desktop-cards'), '--settings', 'calendar'],
                check=True)

    def test_second_request_focuses_existing_window_instead_of_launching_another(self):
        with self.config.with_name('settings.lock').open('w') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with patch.object(controller.subprocess, 'run') as run:
                controller.settings('weather')
                run.assert_not_called()
        self.assertEqual(json.loads(self.config.with_name('settings-command.json').read_text()), {'page': 'weather'})

    def test_cli_settings_route_and_weather_shortcut_use_graphical_pages(self):
        with patch.object(controller.sys, 'argv', ['desktop-cards', 'settings', '--page', 'projects']):
            with patch.object(controller, 'settings') as settings:
                controller.main()
                settings.assert_called_once_with('projects')
        with patch.object(controller, 'settings') as settings:
            controller.weather_picker()
            settings.assert_called_once_with('weather')

    def test_every_native_settings_page_is_reachable_from_the_controller(self):
        source = (Path(__file__).parents[1] / 'src/settings.rs').read_text()
        keys = re.search(r'fn key\(self\) -> &\'static str \{(.*?)\n    \}', source, re.S).group(1)
        native = set(re.findall(r'=> "([a-z-]+)"', keys))
        self.assertIn('desktop-motion', native)
        self.assertEqual(native - set(controller.SETTINGS_PAGES), set())

    def test_desktop_subpage_deep_links_and_old_desktop_page_all_route(self):
        for page in ('desktop-dock', 'desktop-appearance', 'desktop-wallpaper', 'desktop-icons',
                     'desktop-profiles', 'desktop-windows', 'desktop-motion',
                     'desktop-workspaces', 'desktop-window-rules', 'desktop-shortcuts',
                     'desktop-automation', 'desktop',
                     'appearance'):
            with patch.object(controller.sys, 'argv', ['desktop-cards', 'settings', '--page', page]):
                with patch.object(controller, 'settings') as settings:
                    controller.main()
                    settings.assert_called_once_with(page)

    def test_tool_pages_toggles_and_focus_commands_reach_the_desktop(self):
        for page in ('music', 'notes', 'focus'):
            with patch.object(controller.sys, 'argv', ['desktop-cards', 'settings', '--page', page]):
                with patch.object(controller, 'settings') as settings:
                    controller.main()
                    settings.assert_called_once_with(page)
        with patch.dict(controller.os.environ, {'XDG_RUNTIME_DIR': self.directory.name}):
            command = Path(self.directory.name) / 'halcyon/command.json'
            with patch.object(controller.sys, 'argv', ['desktop-cards', 'toggle', 'notes']):
                controller.main()
            self.assertEqual(json.loads(command.read_text()), {'action': 'toggle', 'widget': 'notes'})
            with patch.object(controller.sys, 'argv', ['desktop-cards', 'focus', 'toggle']):
                controller.main()
            self.assertEqual(json.loads(command.read_text()), {'action': 'focus', 'do': 'toggle'})
            with patch.object(controller.sys, 'argv', ['desktop-cards', 'focus', 'explode']):
                with self.assertRaises(SystemExit), redirect_stdout(io.StringIO()), \
                        patch.object(controller.sys, 'stderr', io.StringIO()):
                    controller.main()
        self.assertFalse(self.config.exists(), 'tool commands never edit preferences')

    def test_palette_only_keeps_wallpaper_and_unrelated_preferences(self):
        data = {'theme': 'lavender', 'appearance': {'match_wallpaper': False, 'opacity': 80,
                                                  'future_tint': 'warm'},
                'clock': {'show_date': False}, 'future_setting': 42}
        self.config.write_text(json.dumps(data))
        controller.BACKGROUND.write_text('(source: Color(Single((0.1, 0.2, 0.3))))\n')
        controller.CUSTOM.write_text('["/previous.png",]\n')
        before = {path: path.read_bytes() for path in (controller.BACKGROUND, controller.CUSTOM)}
        with patch.object(controller, 'bundled_wallpaper') as wallpaper, redirect_stdout(io.StringIO()):
            controller.apply_theme('teal')
            wallpaper.assert_not_called()
        self.run.assert_not_called()
        data['theme'] = 'teal'
        self.assertEqual(json.loads(self.config.read_text()), data)
        self.assertEqual({path: path.read_bytes() for path in before}, before)

    def test_enabled_cosmic_matching_runs_after_the_palette_is_saved(self):
        self.config.write_text(json.dumps({'theme': 'lavender', 'appearance': {
            'match_wallpaper': False, 'match_cosmic_colors': True}, 'future_setting': 42}))
        def applied(args, **kwargs):
            self.assertEqual(json.loads(self.config.read_text())['theme'], 'rose')
            return subprocess.CompletedProcess(args, 0, '', '')
        self.run.side_effect = applied
        with redirect_stdout(io.StringIO()):
            controller.apply_theme('rose')
        self.assert_one_link_call('rose', '--colors')
        self.assertEqual(json.loads(self.config.read_text())['future_setting'], 42)

    def test_explicit_cli_palette_exits_desktop_following_and_preserves_legacy_link_choice(self):
        self.config.write_text(json.dumps({'theme': 'teal', 'appearance': {
            'color_source': 'desktop', 'accent': [4, 45, 67], 'match_wallpaper': False,
            'match_cosmic_colors': True, 'opacity': 83}, 'future_setting': 42}))
        with redirect_stdout(io.StringIO()):
            controller.apply_theme('ocean')
        saved = json.loads(self.config.read_text())
        self.assertEqual(saved['theme'], 'ocean')
        self.assertEqual(saved['appearance'], {
            'color_source': 'custom', 'match_wallpaper': False,
            'match_cosmic_colors': True, 'opacity': 83})
        self.assertEqual(saved['future_setting'], 42)
        self.assert_one_link_call('ocean', '--colors')

    def test_failed_cosmic_matching_restores_palette_wallpaper_and_preference(self):
        original = {self.config: b'{"theme":"lavender","appearance":{"match_cosmic_colors":true}}\n',
                    controller.BACKGROUND: b'(source: Path("/previous.png"))\n',
                    controller.CUSTOM: b'["/previous.png",]\n'}
        for path, content in original.items():
            path.write_bytes(content)
        picture = controller.WALLPAPERS / 'ocean-4k.png'
        def failed(args, **kwargs):
            # Issued last: the palette and the collection are already saved.
            self.assertEqual(json.loads(self.config.read_text())['theme'], 'ocean')
            self.assertIn(json.dumps(str(picture)), controller.CUSTOM.read_text())
            return subprocess.CompletedProcess(args, 1, '', 'theme write failed')
        self.run.side_effect = failed
        with patch.object(controller, 'bundled_wallpaper', return_value=picture):
            with self.assertRaisesRegex(RuntimeError, 'theme write failed'):
                controller.apply_theme('ocean')
        self.assert_one_link_call('ocean', '--wallpaper', '--colors')
        self.assertEqual({path: path.read_bytes() for path in original}, original)

    def test_cosmic_switch_applies_current_palette_and_disabling_leaves_system_colors(self):
        initial = {'theme': 'graphite', 'appearance': {'opacity': 80}, 'future_setting': 42}
        self.config.write_text(json.dumps(initial))
        with patch.object(controller, 'apply_cosmic_colors') as sync, redirect_stdout(io.StringIO()):
            controller.set_cosmic_colors(True)
            sync.assert_called_once_with('graphite')
            sync.reset_mock()
            controller.set_cosmic_colors(False)
            sync.assert_not_called()
        initial['appearance']['match_cosmic_colors'] = False
        self.assertEqual(json.loads(self.config.read_text()), initial)

    def test_failed_cosmic_enable_preserves_original_config_bytes(self):
        self.config.write_text('{"theme":"rose","appearance":{"opacity":85}}\n')
        original = self.config.read_bytes()
        with patch.object(controller, 'apply_cosmic_colors', side_effect=RuntimeError('cannot apply')):
            with self.assertRaisesRegex(RuntimeError, 'cannot apply'):
                controller.set_cosmic_colors(True)
        self.assertEqual(self.config.read_bytes(), original)

    def test_preview_palette_and_cosmic_preferences_never_change_system_settings(self):
        self.config.write_text('{"theme":"lavender","appearance":{"match_cosmic_colors":true}}')
        with patch.dict(controller.os.environ, {'DESKTOP_CARDS_CONFIG': str(self.config)}):
            with patch.object(controller, 'bundled_wallpaper') as wallpaper, patch.object(controller, 'apply_cosmic_colors') as sync:
                with redirect_stdout(io.StringIO()):
                    controller.apply_theme('teal')
                    controller.set_cosmic_colors(True)
                wallpaper.assert_not_called()
                sync.assert_not_called()
        self.run.assert_not_called()
        self.assertEqual(json.loads(self.config.read_text())['theme'], 'teal')

    def test_invalid_cosmic_preference_cannot_apply_a_wallpaper(self):
        self.config.write_text('{"appearance":{"match_cosmic_colors":"yes"}}')
        before = self.config.read_bytes()
        with patch.object(controller, 'bundled_wallpaper') as wallpaper:
            with self.assertRaises(RuntimeError):
                controller.apply_theme('teal')
            wallpaper.assert_not_called()
        self.run.assert_not_called()
        self.assertEqual(self.config.read_bytes(), before)

    def test_legacy_theme_command_still_matches_wallpaper(self):
        picture = controller.WALLPAPERS / 'amber-4k.png'
        with patch.object(controller.sys, 'argv', ['desktop-cards', 'theme', 'amber']):
            with patch.object(controller, 'bundled_wallpaper', return_value=picture) as wallpaper:
                with redirect_stdout(io.StringIO()):
                    controller.main()
            wallpaper.assert_called_once_with('amber')
        self.assert_one_link_call('amber', '--wallpaper')
        self.assertEqual(json.loads(self.config.read_text())['theme'], 'amber')
        self.assertFalse(controller.BACKGROUND.exists(), 'the background is written by the helper')

    def test_new_palettes_preserve_wallpaper_when_matching_is_disabled(self):
        original_background = '(source: Path("/personal-wallpaper.png"))\n'
        original_collection = '["/personal-wallpaper.png",]\n'
        controller.BACKGROUND.write_text(original_background)
        controller.CUSTOM.write_text(original_collection)
        for name in ('ocean', 'rose', 'graphite'):
            with self.subTest(theme=name):
                self.config.write_text(json.dumps({
                    'theme': 'lavender', 'appearance': {'match_wallpaper': False, 'opacity': 85},
                    'widget_positions': {'weather': [0.7, 0.3]}, 'future_setting': 42,
                }))
                with redirect_stdout(io.StringIO()):
                    controller.apply_theme(name)
                self.run.assert_not_called()
                saved = json.loads(self.config.read_text())
                self.assertEqual(saved['theme'], name)
                self.assertEqual(saved['appearance'], {'match_wallpaper': False, 'opacity': 85})
                self.assertEqual(saved['widget_positions'], {'weather': [0.7, 0.3]})
                self.assertEqual(saved['future_setting'], 42)
                self.assertEqual(controller.BACKGROUND.read_text(), original_background)
                self.assertEqual(controller.CUSTOM.read_text(), original_collection)

    def test_new_palettes_apply_matching_wallpapers_and_missing_assets_preserve_state(self):
        controller.WALLPAPERS.mkdir()
        controller.BACKGROUND.write_text('(source: Path("/previous.png"))\n')
        controller.CUSTOM.write_text('["/previous.png",]\n')
        self.config.write_text('{"theme":"lavender","future_setting":42}\n')
        background = controller.BACKGROUND.read_bytes()
        for name in ('ocean', 'rose', 'graphite'):
            with self.subTest(theme=name):
                self.run.reset_mock()
                wallpaper = controller.WALLPAPERS / f'{name}-4k.png'
                wallpaper.write_bytes(b'wallpaper fixture')
                with redirect_stdout(io.StringIO()):
                    controller.apply_theme(name)
                self.assert_one_link_call(name, '--wallpaper')
                self.assertEqual(controller.BACKGROUND.read_bytes(), background)
                self.assertIn(json.dumps(str(wallpaper)), controller.CUSTOM.read_text())
                saved = json.loads(self.config.read_text())
                self.assertEqual(saved['theme'], name)
                self.assertEqual(saved['future_setting'], 42)
                wallpaper.unlink()
                before = {path: path.read_bytes() for path in
                          (self.config, controller.BACKGROUND, controller.CUSTOM)}
                self.run.reset_mock()
                with patch.object(controller, 'bundled_wallpaper', side_effect=RuntimeError('Bundled wallpapers are missing')):
                    with self.assertRaisesRegex(RuntimeError, 'Bundled wallpapers are missing'):
                        controller.apply_theme(name)
                self.run.assert_not_called()
                self.assertEqual({path: path.read_bytes() for path in before}, before)

    def test_a_missing_built_in_wallpaper_asks_to_reinstall_without_touching_anything(self):
        controller.BACKGROUND.write_text('(source: Path("/previous.png"))\n')
        self.config.write_text('{"theme":"lavender","future_setting":42}')
        with patch.object(controller.urllib.request, 'urlopen') as network:
            with self.assertRaisesRegex(RuntimeError, 'Reinstall Halcyon'):
                controller.apply_theme('ocean')
            network.assert_not_called()
        self.assertEqual(controller.BACKGROUND.read_text(), '(source: Path("/previous.png"))\n')
        self.assertEqual(json.loads(self.config.read_text())['future_setting'], 42)

    def test_invalid_config_does_not_change_wallpaper_or_overwrite_settings(self):
        for invalid in ('{ unfinished', '[]', '{"appearance": {"match_wallpaper": "false"}}'):
            with self.subTest(invalid=invalid):
                self.config.write_text(invalid)
                with patch.object(controller, 'bundled_wallpaper') as wallpaper:
                    with self.assertRaises(RuntimeError):
                        controller.apply_theme('teal')
                    wallpaper.assert_not_called()
                self.run.assert_not_called()
                self.assertEqual(self.config.read_text(), invalid)

    def test_concurrent_config_writers_preserve_every_update(self):
        barrier = threading.Barrier(8)
        def writer(index):
            barrier.wait()
            def change(data):
                count = data.get('counter', 0)
                time.sleep(0.002)
                data['counter'] = count + 1
                data[f'writer_{index}'] = True
            controller.edit_config(change)
        with ThreadPoolExecutor(max_workers=8) as workers:
            list(workers.map(writer, range(8)))
        data = json.loads(self.config.read_text())
        self.assertEqual(data['counter'], 8)
        self.assertTrue(all(data[f'writer_{i}'] for i in range(8)))
        self.assertTrue(Path(str(self.config) + '.lock').exists())

    def test_failed_atomic_write_keeps_previous_file_and_cleans_temporary_file(self):
        self.config.write_text('previous')
        with patch.object(controller.os, 'replace', side_effect=OSError('cannot replace')):
            with self.assertRaises(OSError):
                controller.atomic(self.config, 'new')
        self.assertEqual(self.config.read_text(), 'previous')
        self.assertEqual(list(self.config.parent.iterdir()), [self.config])

    def test_failed_theme_change_restores_wallpaper_collection_and_config(self):
        controller.WALLPAPERS.mkdir()
        (controller.WALLPAPERS / 'amber-4k.png').write_bytes(b'wallpaper fixture')
        original = {self.config: b'{"theme":"lavender","future_setting":42}\n',
                    controller.BACKGROUND: b'(source: Path("/previous.png"))\n',
                    controller.CUSTOM: b'["/previous.png",]\n'}
        real_atomic = controller.atomic
        for failed_path in [controller.CUSTOM, self.config, 'helper']:
            with self.subTest(failed_path=getattr(failed_path, 'name', failed_path)):
                self.run.reset_mock()
                for path, content in original.items():
                    path.write_bytes(content)
                    path.chmod(0o640)
                def fail_selected_write(path, value):
                    if path == failed_path:
                        raise OSError('injected write failure')
                    return real_atomic(path, value)
                self.run.return_value = subprocess.CompletedProcess(
                    [], 1 if failed_path == 'helper' else 0, '', 'Background is set elsewhere')
                with patch.object(controller, 'atomic', fail_selected_write):
                    with self.assertRaises((OSError, RuntimeError)):
                        controller.apply_theme('amber')
                if failed_path == 'helper':
                    self.assert_one_link_call('amber', '--wallpaper')
                else:
                    self.run.assert_not_called()
                for path, content in original.items():
                    self.assertEqual(path.read_bytes(), content)
                    self.assertEqual(path.stat().st_mode & 0o777, 0o640)
                self.assertEqual((controller.CONFIG / 'original-background.ron').read_bytes(),
                                 original[controller.BACKGROUND])
                self.assertEqual((controller.CONFIG / 'original-custom-images.ron').read_bytes(),
                                 original[controller.CUSTOM])

    def test_restore_uses_store_helper(self):
        saved = controller.CONFIG / 'original-background.ron'
        with patch.object(controller, 'service') as service:
            with self.assertRaisesRegex(RuntimeError, 'No previous wallpaper backup'):
                controller.restore()
            self.run.assert_not_called()
            saved.write_text('(output: "all", source: Path("/previous.png"))\n')
            with redirect_stdout(io.StringIO()):
                controller.restore()
            self.run.assert_called_once_with([self.helper, '--restore-background', str(saved)],
                                             text=True, capture_output=True)
            service.assert_called_once_with('stop')
            self.assertFalse(controller.BACKGROUND.exists(), 'the helper writes the background')
            service.reset_mock()
            self.run.return_value = subprocess.CompletedProcess([], 1, '', 'not a valid background')
            with self.assertRaisesRegex(RuntimeError, 'not a valid background'):
                controller.restore()
            service.assert_not_called()

    def test_restore_wallpaper_leaves_widgets_startup_and_preferences_unchanged(self):
        saved = controller.CONFIG / 'original-background.ron'
        saved.write_text('(output: "all", source: Path("/previous.png"))\n')
        preferences = '{"enabled_widgets":["calendar","clock"],"theme":"teal"}\n'
        self.config.write_text(preferences)
        with patch.object(controller, 'service') as service, redirect_stdout(io.StringIO()):
            with patch.object(controller.sys, 'argv', ['desktop-cards', 'restore-wallpaper']):
                controller.main()
            # The sole subprocess restores the background through the safe store.
            # In particular, no systemctl disable/stop command changes startup or visibility.
            self.run.assert_called_once_with([self.helper, '--restore-background', str(saved)],
                                             text=True, capture_output=True)
            service.assert_not_called()
            self.assertEqual(self.config.read_text(), preferences)
            self.assertFalse(controller.BACKGROUND.exists(), 'the helper writes the background')

    def test_restore_wallpaper_failures_leave_widgets_and_startup_unchanged(self):
        saved = controller.CONFIG / 'original-background.ron'
        with patch.object(controller, 'service') as service:
            with self.assertRaisesRegex(RuntimeError, 'No previous wallpaper backup'):
                controller.restore_wallpaper()
            self.run.assert_not_called()
            saved.write_text('(output: "all", source: Path("/previous.png"))\n')
            self.run.return_value = subprocess.CompletedProcess([], 1, '', 'wallpaper changed elsewhere')
            with self.assertRaisesRegex(RuntimeError, 'wallpaper changed elsewhere'):
                controller.restore_wallpaper()
            self.run.assert_called_once_with([self.helper, '--restore-background', str(saved)],
                                             text=True, capture_output=True)
            service.assert_not_called()

    def test_wallpapers_install_checks_the_installed_wallpapers_offline(self):
        for name in controller.THEMES:
            controller.wallpaper_file(name).parent.mkdir(parents=True, exist_ok=True)
            controller.wallpaper_file(name).write_bytes(b'wallpaper')
        output = io.StringIO()
        with patch.object(controller.sys, 'argv', ['halcyon-ctl', 'wallpapers-install']):
            with patch.object(controller.urllib.request, 'urlopen') as network, redirect_stdout(output):
                controller.main()
                network.assert_not_called()
        self.run.assert_not_called()
        self.assertIn('installed', output.getvalue())
