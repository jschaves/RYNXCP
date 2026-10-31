# Minimal Go application

A static binary using the Go standard library.

## Application file

Create `main.go`:

```go
package main

import (
	"fmt"
	"net/http"
	"os"
)

func main() {
	host := os.Getenv("HOST")
	if host == "" {
		host = "127.0.0.1"
	}
	port := os.Getenv("PORT")
	if port == "" {
		port = "8000"
	}

	http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		fmt.Fprintln(w, "Hola Mundo")
	})
	http.ListenAndServe(host+":"+port, nil)
}
```

Build the Linux binary on a development machine:

```bash
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o server .
```

Upload `server` by FTP to:

```text
/home/<account>/apps/hello-go/server
```

Make the uploaded file executable (`0755`), for example with the FTP client's
`SITE CHMOD 755 server` command. The panel does not execute non-executable
files.

## Panel configuration

Create it from **Applications → New application**:

| Field | Value |
| --- | --- |
| Name | `hello-go` |
| Runtime | Go |
| Version | Not applicable |
| Internal port | `8000` |
| Entrypoint | `server` |
| Command | `server` |
| Arguments | Empty |
| Start application | Enabled |

The command is relative to the application working directory, so systemd runs
`/home/<account>/apps/hello-go/server`.

## Publish it

Create a website with runtime **Go**, select `hello-go` as the application, and
use the default document root (`<domain>/public_html`). The application and the website count as
two resources toward `web_domains`.

See the [Python guide](python-app.md#troubleshooting) for the common 502 checks.
