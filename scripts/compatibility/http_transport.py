"""Cancellable HTTPS body transport. Authentication stays in archive_io.

All DNS/connect/TLS/headers/chunk parsing runs in an owned subprocess, never in
an uninterruptible caller thread. stdout is body bytes only; exit codes carry
sanitized failures. No successful EOF is returned before the worker exits zero.
"""
from contextlib import contextmanager
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import time
import urllib.parse
import urllib.request

CHUNK = 1024 * 1024
CLEANUP_SECONDS = 1


def run_worker(url, opener=None):
    """opener is a local test seam; the production CLI always uses urlopen."""
    if urllib.parse.urlparse(url).scheme != 'https':
        return 2
    try:
        with (opener or urllib.request.urlopen)(url, timeout=30) as response:
            if urllib.parse.urlparse(response.url).scheme != 'https':
                return 3
            while block := response.read1(CHUNK):
                sys.stdout.buffer.write(block)
                sys.stdout.buffer.flush()
        return 0
    except Exception:
        # Never emit URLs, credentials, server bytes or exception text to logs.
        return 4


def start_worker(url):
    return subprocess.Popen([sys.executable, '-I', str(Path(__file__).resolve()), url],
                            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.DEVNULL, start_new_session=True)


class BodyReader:
    def __init__(self, process, deadline, timeout_code):
        self.process, self.deadline, self.timeout_code = process, deadline, timeout_code
        self.finished = False

    def remaining(self):
        remaining = self.deadline - time.monotonic()
        if remaining <= 0:
            raise ValueError(self.timeout_code)
        return remaining

    def read(self, size):
        if size == 0:
            return b''
        if size < 0:
            raise ValueError('unbounded_transport_read')
        self.remaining()
        if self.finished:
            return b''
        if not select.select([self.process.stdout], [], [], self.remaining())[0]:
            raise ValueError(self.timeout_code)
        self.remaining()
        data = os.read(self.process.stdout.fileno(), min(size, CHUNK))
        if data:
            return data
        try:
            status = self.process.wait(timeout=self.remaining())
        except subprocess.TimeoutExpired as error:
            raise ValueError(self.timeout_code) from error
        self.remaining()
        if status:
            raise ValueError({2: 'https_required', 3: 'insecure_redirect'}.get(status, 'https_transport_error'))
        self.finished = True
        return b''


@contextmanager
def open_https(url, deadline=None, timeout_code='metadata_timeout'):
    if urllib.parse.urlparse(url).scheme != 'https':
        raise ValueError('https_required')
    deadline = time.monotonic() + 300 if deadline is None else deadline
    if time.monotonic() >= deadline:
        raise ValueError(timeout_code)
    process = start_worker(url)
    reader = BodyReader(process, deadline, timeout_code)
    try:
        yield reader
    finally:
        # No production threads or response/socket objects survive this scope.
        # The deadline includes transport work; kill/reap has a separate 1s
        # cleanup bound. Do not touch a process group already reaped at EOF.
        if process.returncode is None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        try:
            process.wait(timeout=CLEANUP_SECONDS)
        finally:
            process.stdout.close()


if __name__ == '__main__':
    sys.exit(run_worker(sys.argv[1]))
