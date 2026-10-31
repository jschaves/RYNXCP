# Minimal Java application

A small JAR using the JDK's built-in HTTP server. Build it with a JDK; the
server itself only needs Java to run.

## Application file

Create `App.java`:

```java
import com.sun.net.httpserver.HttpServer;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;

public class App {
    public static void main(String[] args) throws Exception {
        String host = System.getenv().getOrDefault("HOST", "127.0.0.1");
        int port = Integer.parseInt(System.getenv().getOrDefault("PORT", "8000"));
        HttpServer server = HttpServer.create(new InetSocketAddress(host, port), 0);
        server.createContext("/", exchange -> {
            byte[] body = "Hola Mundo\n".getBytes(StandardCharsets.UTF_8);
            exchange.getResponseHeaders().add("Content-Type", "text/plain; charset=utf-8");
            exchange.sendResponseHeaders(200, body.length);
            exchange.getResponseBody().write(body);
            exchange.close();
        });
        server.start();
    }
}
```

Build the JAR on a development machine with a JDK:

```bash
javac -d build App.java
jar --create --file app.jar --main-class App -C build .
```

Upload `app.jar` by FTP to:

```text
/home/<account>/apps/hello-java/app.jar
```

## Panel configuration

Create it from **Applications → New application**:

| Field | Value |
| --- | --- |
| Name | `hello-java` |
| Runtime | Java |
| Version | Installed Java version |
| Internal port | `8000` |
| Entrypoint | `app.jar` |
| Command | `java` |
| Arguments | `-jar,app.jar` |
| Start application | Enabled |

The arguments field is comma-separated; `-jar` and `app.jar` become separate
systemd arguments.

## Publish it

Create a website with runtime **Java**, select `hello-java` as the application,
and use the default document root (`<domain>/public_html`). The application and the website
count as two resources toward `web_domains`.

See the [Python guide](python-app.md#troubleshooting) for the common 502 checks.
