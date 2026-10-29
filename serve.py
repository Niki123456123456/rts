from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class Handler(SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


if __name__ == "__main__":
    public = Path(__file__).resolve().parent / "public"
    ThreadingHTTPServer(("127.0.0.1", 18765), partial(Handler, directory=str(public))).serve_forever()
