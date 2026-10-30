# Minimal Node.js application

A dependency-free HTTP server for testing a managed Node.js application.

## Application file

Create `server.js` in the application directory:

```js
const http = require('node:http');

const host = process.env.HOST || '127.0.0.1';
const port = Number(process.env.PORT || 8000);

http.createServer((request, response) => {
  response.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8' });
  response.end('Hola Mundo\n');
}).listen(port, host);
```

## Panel configuration

Create it from **Applications → New application**:

| Field | Value |
| --- | --- |
| Name | `hello-node` |
| Runtime | Node.js |
| Version | Installed Node.js version |
| Internal port | `8000` |
| Entrypoint | `server.js` |
| Command | `node` |
| Arguments | `server.js` |
| Start application | Enabled |

Upload `server.js` by FTP to:

```text
/home/<account>/apps/hello-node/server.js
```

The application must bind to `127.0.0.1:8000`. A process that only prints text
exits immediately and Nginx returns **502 Bad Gateway**.

## Publish it

Create a website with runtime **Node.js**, select `hello-node` as the
application, and use `public_html` as the document root. The application and the
website count as two resources toward `web_domains`.

See the [Python guide](python-app.md#troubleshooting) for the common 502 checks.
