# Minimal Python application

This example publishes a small Python HTTP application through RYNXCP without
external Python dependencies. It is useful to verify the full path from an
account application to the public website.

## 1. Create the application

Log in as the account operator (or let an administrator impersonate the
account), then open **Applications → New application** and use:

| Field | Value |
| --- | --- |
| Account | The hosting account |
| Name | `hello-python` |
| Runtime | Python |
| Version | The installed Python version |
| Internal port | `8000` |
| Entrypoint | `app.py` |
| Command | `python3` |
| Arguments | `app.py` |
| Start application | Enabled |

The application working directory is:

```text
/home/<account>/apps/<application>/
```

For this example:

```text
/home/<account>/apps/hello-python/app.py
```

## 2. Upload `app.py`

Create the file through the account FTP user (the FTP session is chrooted to the
account home). The panel file manager only handles a website document root, not
the `apps/` directory.

```python
import os
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"Hola Mundo\n"
        self.send_response(200)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


host = os.environ.get("HOST", "127.0.0.1")
port = int(os.environ.get("PORT", "8000"))
ThreadingHTTPServer((host, port), Handler).serve_forever()
```

The systemd unit sets `HOST=127.0.0.1` and `PORT` to the internal port selected
in the panel. The process must stay running and listen on loopback; a script
that only prints text exits immediately and Nginx returns **502 Bad Gateway**.

## 3. Publish it as a website

Open **Websites → New website**:

| Field | Value |
| --- | --- |
| Domain | The public domain or subdomain |
| Account | The same hosting account |
| Runtime | Python |
| Application | `hello-python` |
| Document root | `<domain>/public_html` |
| TLS | As needed |
| Redirect HTTP to HTTPS | Optional |

The website adds an Nginx reverse proxy to the application port. Both the
application and the website count toward the package's `web_domains` limit.

Open the domain after the application is active. The expected response is:

```text
Hola Mundo
```

## Gunicorn and Uvicorn

For Flask or FastAPI applications, the panel can run `gunicorn` or `uvicorn`,
but it does not install Python packages automatically. Those commands and the
application dependencies must already be available to the account's Python.

Arguments are comma-separated in the application form:

- Gunicorn WSGI: command `gunicorn`, arguments
  `app:app,--bind=127.0.0.1:8000`
- Uvicorn ASGI: command `uvicorn`, arguments
  `app:app,--host,127.0.0.1,--port,8000`

## Troubleshooting

- **502 Bad Gateway:** the application is stopped, crashed or is not listening
  on the configured loopback port.
- **Application stopped:** check that `app.py` exists in the application
  directory and that the command and arguments are correct.
- **Port mismatch:** the application port and the website proxy port must be the
  same.
- **Python dependencies:** install them in the environment used by `python3`,
  or use a standard-library application such as the example above.
