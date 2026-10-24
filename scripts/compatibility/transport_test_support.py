"""Local subprocess/HTTPResponse fixtures. No listener or external requests."""
from contextlib import contextmanager
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import threading
from unittest.mock import patch

import http_transport

HERE = Path(__file__).resolve().parent


def spawn(code, args, pass_fds=()):
    return subprocess.Popen([sys.executable, '-I', '-c', code, str(HERE), *map(str, args)],
                            pass_fds=pass_fds, stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                            start_new_session=True)


@contextmanager
def response_worker(data, final_url='https://fixture/source', delay=0):
    """Replace the worker's opener, keeping the real process/pipe boundary."""
    children = []
    with tempfile.TemporaryDirectory() as work:
        path = Path(work)/'body';path.write_bytes(data)
        code = '''import sys,time,io
sys.path.insert(0,sys.argv[1])
from http_transport import run_worker
from pathlib import Path
def opener(url,timeout):
    time.sleep(float(sys.argv[4]))
    response=io.BytesIO(Path(sys.argv[2]).read_bytes());response.url=sys.argv[3]
    return response
sys.exit(run_worker(sys.argv[5],opener))
'''
        def start(url):
            child=spawn(code,[path,final_url,delay,url]);children.append(child);return child
        with patch.object(http_transport,'start_worker',side_effect=start):
            yield children
        assert all(child.poll() is not None and child.stdout.closed for child in children)


@contextmanager
def wire_worker(phase, *, socket_timeout=30, ticks=330, interval=.1):
    """Real HTTPResponse parsing in the production worker, AF_UNIX transport.

    Headers, chunk extensions and trailers deliberately take ~33s to complete
    without cancellation. The caller's one-second budget must interrupt them.
    """
    left,right=socket.socketpair();stop=threading.Event();children=[]
    code='''import sys,socket,http.client
sys.path.insert(0,sys.argv[1])
from http_transport import run_worker
sock=socket.socket(fileno=int(sys.argv[2]));sock.settimeout(float(sys.argv[3]))
def opener(url,timeout):
    response=http.client.HTTPResponse(sock);response.begin();response.url=url
    return response
try:sys.exit(run_worker(sys.argv[4],opener))
finally:sock.close()
'''
    def start(url):
        child=spawn(code,[left.fileno(),socket_timeout,url],(left.fileno(),))
        children.append(child);return child
    def send():
        try:
            if phase=='headers':
                right.sendall(b'HTTP/1.1 200 OK\r\nX-Slow: ')
            else:
                right.sendall(b'HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n')
                right.sendall(b'1;' if phase=='chunk' else b'1\r\nx\r\n0\r\nX-Slow: ')
            for _ in range(ticks):
                right.sendall(b'x')
                if stop.wait(interval):return
            right.sendall(b'\r\nContent-Length: 1\r\n\r\nx' if phase=='headers' else
                          b'\r\nx\r\n0\r\n\r\n' if phase=='chunk' else b'\r\n\r\n')
        except OSError:pass
        finally:right.close()
    sender=threading.Thread(target=send,name='fixture-http-sender');sender.start()
    try:
        with patch.object(http_transport,'start_worker',side_effect=start):
            yield children
    finally:
        stop.set();left.close();right.close();sender.join(timeout=1)
        assert not sender.is_alive()
        assert all(child.poll() is not None and child.stdout.closed for child in children)
