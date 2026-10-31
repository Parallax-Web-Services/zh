#!/usr/bin/env python3
"""Dependency-free browser preview of the resource map (no zh runtime).

NOT a production agent, remote execution service, or Positronic control plane.
Binds loopback only, with a random token for telemetry access.
"""

import argparse
import json
import os
import secrets
import shutil
import threading
import time
import urllib.parse
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "apps" / "resource-map"
TOKEN = secrets.token_hex(32)
CPU_LOCK = threading.Lock()
PREV_CPU = None


def cpu_usage():
    global PREV_CPU
    try:
        values = [int(x) for x in Path('/proc/stat').read_text().splitlines()[0].split()[1:]]
        total = sum(values)
        idle = values[3] + values[4]
        with CPU_LOCK:
            previous = PREV_CPU
            PREV_CPU = (total, idle)
        if previous and total > previous[0]:
            return max(0, min(100, 100 * (1 - (idle - previous[1]) / (total - previous[0]))))
    except (OSError, ValueError, IndexError, ZeroDivisionError):
        pass
    return None


def get_uptime():
    try:
        return int(float(Path('/proc/uptime').read_text().split()[0]))
    except (OSError, ValueError, IndexError):
        return None


def local_snapshot():
    data = Path('/proc/meminfo').read_text()
    fields = {}
    for line in data.splitlines():
        name, _, rest = line.partition(':')
        try:
            fields[name] = int(rest.split()[0]) * 1024
        except (ValueError, IndexError):
            continue
    memory_total = fields.get('MemTotal', 0)
    memory_free = min(memory_total, fields.get('MemAvailable', 0))
    disk = shutil.disk_usage('/')
    interfaces = []
    for interface in sorted(Path('/sys/class/net').iterdir()):
        if interface.name == 'lo':
            continue
        try:
            state = (interface / 'operstate').read_text().strip()
        except OSError:
            state = 'unknown'
        interfaces.append({'name': interface.name, 'link_state': state})
    try:
        load_one = os.getloadavg()[0]
    except OSError:
        load_one = None
    release = os.uname().release.lower()
    return {
        'schema_version': 1, 'collected_at_ms': round(time.time() * 1000),
        'node': {
            'name': os.uname().nodename,
            'operating_system': 'linux',
            'is_wsl': 'microsoft' in release,
            'uptime_seconds': get_uptime(),
        },
        'cpu': {'logical_cores': os.cpu_count() or 1, 'usage_percent': cpu_usage(), 'load_one_minute': load_one},
        'memory': {'total_bytes': memory_total, 'available_bytes': memory_free, 'used_bytes': memory_total - memory_free},
        'storage': {'mount': '/', 'total_bytes': disk.total, 'available_bytes': disk.free, 'used_bytes': disk.used},
        'network': interfaces,
        'gpu': {'status': 'unavailable', 'note': 'GPU discovery not implemented in alpha'},
        'fabric': {'mode': 'local_only', 'connected_peers': [], 'orchestration_status': 'not_implemented'},
    }


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.headers.get('Host') != f'127.0.0.1:{self.server.server_port}':
            self.send_error(403, 'Invalid host')
            return
        route = urllib.parse.urlsplit(self.path).path
        if route == '/api/v1/local':
            if not secrets.compare_digest(self.headers.get('Authorization', ''), f'Bearer {TOKEN}'):
                self.send_error(401, 'Unauthorized')
                return
            body = json.dumps(local_snapshot()).encode()
            mime = 'application/json'
        elif route in ('/', '/index.html', '/app.js', '/styles.css'):
            name = {'/': 'index.html'}.get(route, route.removeprefix('/'))
            body = (ASSETS / name).read_bytes()
            mime = {'index.html': 'text/html; charset=utf-8', 'app.js': 'application/javascript; charset=utf-8', 'styles.css': 'text/css; charset=utf-8'}[name]
        else:
            self.send_error(404)
            return
        self.send_response(200)
        self.send_header('Content-Type', mime)
        self.send_header('Content-Length', str(len(body)))
        self.send_header('Cache-Control', 'no-store')
        self.send_header('X-Frame-Options', 'DENY')
        self.send_header('Referrer-Policy', 'no-referrer')
        self.send_header('Content-Security-Policy', "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; base-uri 'none'; frame-ancestors 'none'")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        # Do not leak session data in access logs.
        return


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--no-open', action='store_true', help='Print URL without launching a browser')
    args = parser.parse_args()
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    url = f'http://127.0.0.1:{server.server_port}/#token={TOKEN}'
    print('PARALLAX Ж / resource map preview — Rust zh not running', flush=True)
    print(f'Open: {url}', flush=True)
    print('Ctrl+C to stop. Loopback only; no remote commands enabled.', flush=True)
    if not args.no_open:
        webbrowser.open(url)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == '__main__':
    main()
