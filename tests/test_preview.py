import importlib.util
import json
import threading
import unittest
import urllib.error
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('zh_preview', ROOT / 'scripts' / 'preview.py')
preview = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(preview)


class PreviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = ThreadingHTTPServer(('127.0.0.1', 0), preview.Handler)
        cls.base = f'http://127.0.0.1:{cls.server.server_port}'
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=2)

    def test_ui_html(self):
        with urllib.request.urlopen(self.base) as response:
            self.assertIn(b'Positronic', response.read())

    def test_unauthorized_api(self):
        with self.assertRaises(urllib.error.HTTPError) as caught:
            urllib.request.urlopen(self.base + '/api/v1/local')
        self.assertEqual(caught.exception.code, 401)

    def test_authorized_telemetry(self):
        request = urllib.request.Request(self.base + '/api/v1/local', headers={
            'Authorization': 'Bearer ' + preview.TOKEN,
        })
        with urllib.request.urlopen(request) as response:
            data = json.load(response)
        self.assertEqual(data['schema_version'], 1)
        self.assertEqual(data['fabric']['mode'], 'local_only')
        self.assertEqual(data['fabric']['connected_peers'], [])
        self.assertGreater(data['memory']['total_bytes'], 0)

    def test_dns_rebinding_guard(self):
        request = urllib.request.Request(self.base + '/api/v1/local', headers={
            'Host': 'evil.example', 'Authorization': 'Bearer ' + preview.TOKEN,
        })
        with self.assertRaises(urllib.error.HTTPError) as caught:
            urllib.request.urlopen(request)
        self.assertEqual(caught.exception.code, 403)


if __name__ == '__main__':
    unittest.main()
