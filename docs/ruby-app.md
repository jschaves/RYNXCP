# Minimal Ruby application

A dependency-free HTTP server using the Ruby standard library.

## Application file

Create `app.rb` in the application directory:

```ruby
require "socket"

host = ENV.fetch("HOST", "127.0.0.1")
port = Integer(ENV.fetch("PORT", "8000"))
server = TCPServer.new(host, port)

loop do
  client = server.accept
  client.gets
  body = "Hola Mundo\n"
  response = "HTTP/1.1 200 OK\r\n" \
             "Content-Type: text/plain; charset=utf-8\r\n" \
             "Content-Length: #{body.bytesize}\r\n" \
             "Connection: close\r\n\r\n#{body}"
  client.write(response)
  client.close
end
```

## Panel configuration

Create it from **Applications → New application**:

| Field | Value |
| --- | --- |
| Name | `hello-ruby` |
| Runtime | Ruby |
| Version | Installed Ruby version |
| Internal port | `8000` |
| Entrypoint | `app.rb` |
| Command | `ruby` |
| Arguments | `app.rb` |
| Start application | Enabled |

Upload `app.rb` by FTP to:

```text
/home/<account>/apps/hello-ruby/app.rb
```

The listener uses the `HOST` and `PORT` values injected by systemd.

## Publish it

Create a website with runtime **Ruby**, select `hello-ruby` as the application,
and use the default document root (`<domain>/public_html`). The application and the website
count as two resources toward `web_domains`.

See the [Python guide](python-app.md#troubleshooting) for the common 502 checks.
