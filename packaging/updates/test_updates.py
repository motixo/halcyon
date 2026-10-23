"""Handoff policy tests, not D-Bus/UI or native desktop acceptance."""
import unittest
from status_contract import FAILURES, classify, reduce_status


def installed():
    return dict(running=dict(path='/usr/bin/halcyon', sha256='a'*64, version='0.2.0'),
                package=dict(name='cosmic-ext-halcyon', version='0.2.0', architecture='amd64',
                             status='installed', owns_executable=True, installed_file_sha256='a'*64),
                enrollment='verified-enabled')


class UpdatesContract(unittest.TestCase):
    def test_channel_uses_running_executable_not_other_installation_on_path(self):
        measurement = installed()
        measurement['running']['path'] = '/home/fixture/project/target/release/halcyon'
        self.assertEqual(classify(measurement)[0], 'Unknown')
        measurement['running']['source_install_verified'] = True
        self.assertEqual(classify(measurement)[0], 'Source')
        self.assertEqual(reduce_status(measurement)['action'], 'build-info')

    def test_modified_deleted_or_unknown_executable_is_not_upgraded(self):
        for changed in ({'sha256': 'b'*64}, {'deleted': True}, {'sha256': None}):
            measurement = installed()
            measurement['running'].update(changed)
            result = reduce_status(measurement)
            self.assertEqual(result['channel'], 'Unknown')
            self.assertEqual(result['action'], 'installation-info')

    def test_explicit_enrollment_only(self):
        for state, channel in [('absent', 'ManualDeb'), ('disabled', 'ManualDeb'),
                               ('unknown', 'Unknown'), ('verified-enabled', 'Apt')]:
            measurement = installed()
            measurement['enrollment'] = state
            self.assertEqual(classify(measurement)[0], channel)

    def test_no_check_without_explicit_transaction(self):
        self.assertEqual(reduce_status(installed())['check'], 'not-checked')

    def test_updates_no_update_debian_order_and_other_architecture(self):
        for ids, status in [([], 'up-to-date'),
                            (['cosmic-ext-halcyon;0.2.0~rc1;amd64;archive'], 'up-to-date'),
                            (['cosmic-ext-halcyon;0.2.1;arm64;archive'], 'up-to-date'),
                            (['cosmic-ext-halcyon;0.2.1;amd64;archive'], 'available')]:
            self.assertEqual(reduce_status(installed(), dict(outcome='finished-success', package_ids=ids))['check'], status)

    def test_failure_and_auth_cancel_never_leave_candidate(self):
        for outcome in FAILURES | {'pending'}:
            result = reduce_status(installed(), dict(outcome=outcome,
                package_ids=['cosmic-ext-halcyon;0.2.1;amd64;archive']))
            self.assertIsNone(result['candidate_version'])
            self.assertNotEqual(result['check'], 'up-to-date')

    def test_error_signal_beats_finished_and_malformed_ids(self):
        for transaction in [dict(outcome='finished-success', error_code=1),
                            dict(outcome='finished-success', package_ids=['bad']),
                            dict(outcome='finished-success', package_ids=['cosmic-ext-halcyon;0.2.1;amd64;repo']*2)]:
            self.assertEqual(reduce_status(installed(), transaction)['check'], 'failed')

    def test_absent_islands_stays_absent_and_pending_login_is_separate(self):
        measurement = installed()
        measurement['islands_refresh'] = 'logout_required'
        transaction = dict(outcome='finished-success', package_ids=['cosmic-ext-halcyon-islands;0.2.1;amd64;repo'])
        self.assertFalse(reduce_status(measurement, transaction)['pending_islands_login'])
        measurement['islands_installed_verified'] = True
        result = reduce_status(measurement, transaction)
        self.assertTrue(result['pending_islands_login'])
        self.assertEqual(result['check'], 'up-to-date')


if __name__ == '__main__':
    unittest.main()
