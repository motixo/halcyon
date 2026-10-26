"""Private sockets and stub processes exercise restart without a desktop session."""
import json
import os
from pathlib import Path
import socket
import tempfile
import threading
import unittest
from unittest.mock import patch

import panel_launcher as launcher


class Restart(unittest.TestCase):
    def test_auditfix_second_host_opens_a_fresh_display_connection(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            child = root/'stock-fixture'
            child.write_text('''#!/usr/bin/python3
import json, os, socket, sys
record = {'inherited': os.environ.get('WAYLAND_SOCKET'), 'notify': os.environ.get('PANEL_NOTIFICATIONS_FD')}
if record['inherited'] is not None:
    stream = socket.socket(fileno=int(record['inherited']))
else:
    stream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    stream.settimeout(2)
    stream.connect(os.environ['WAYLAND_DISPLAY'])
stream.settimeout(2)
stream.sendall(b'request')
record['response'] = stream.recv(64).decode()
with open(os.environ['ATTEMPTS'], 'a') as output:
    output.write(json.dumps(record) + '\\n')
sys.exit(23 if record['inherited'] else 0)
''')
            child.chmod(0o755)
            first, inherited = socket.socketpair()
            descriptor = inherited.detach()
            read_fd, notification = os.pipe()
            display = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            display.bind(str(root/'display'))
            display.listen(1)
            display.settimeout(4)
            failures = []
            def server():
                try:
                    with first:
                        first.settimeout(2)
                        self.assertEqual(first.recv(64), b'request')
                        first.sendall(b'protocol-error')
                    connection, _ = display.accept()
                    with connection:
                        connection.settimeout(2)
                        self.assertEqual(connection.recv(64), b'request')
                        connection.sendall(b'ready')
                except BaseException as error:
                    failures.append(error)
            worker = threading.Thread(target=server, daemon=True)
            worker.start()
            try:
                env = {'PATH':'/usr/bin', 'HOME':str(root), 'XDG_RUNTIME_DIR':str(root),
                       'WAYLAND_SOCKET':str(descriptor), 'PANEL_NOTIFICATIONS_FD':str(notification),
                       'WAYLAND_DISPLAY':str(root/'display'), 'ATTEMPTS':str(root/'attempts')}
                with patch.object(launcher, 'compositor_gone', return_value=False), \
                     patch.object(launcher, 'RESTART_DELAYS', (0,)):
                    result = launcher.stock_host(child, (descriptor, notification), limit=2, env=env)
                worker.join(5)
                self.assertFalse(worker.is_alive())
                self.assertEqual(failures, [])
                self.assertEqual(result, 0)
                attempts = [json.loads(line) for line in (root/'attempts').read_text().splitlines()]
                self.assertEqual(len(attempts), 2)
                self.assertEqual(attempts[0]['response'], 'protocol-error')
                self.assertIsNotNone(attempts[0]['notify'])
                self.assertEqual(attempts[1], {'inherited':None, 'notify':None, 'response':'ready'})
            finally:
                display.close()
                first.close()
                for fd in (descriptor, read_fd, notification):
                    try:
                        os.close(fd)
                    except OSError:
                        pass
                worker.join(5)


if __name__ == '__main__':
    unittest.main()
