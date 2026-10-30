# Minimal .NET application

A minimal ASP.NET application published as a DLL and run by the `dotnet` host.

## Project files

Set `TargetFramework` to the .NET version installed on the server. The example
uses `net8.0`; use `net9.0` or the installed SDK version when appropriate.

Create `hello-dotnet.csproj`:

```xml
<Project Sdk="Microsoft.NET.Sdk.Web">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <AssemblyName>app</AssemblyName>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
  </PropertyGroup>
</Project>
```

Create `Program.cs`:

```csharp
var builder = WebApplication.CreateBuilder(args);
var app = builder.Build();

app.MapGet("/", () => "Hola Mundo");

app.Run();
```

Build on a development machine with the .NET SDK:

```bash
dotnet publish -c Release -o publish
```

Upload the **contents of `publish/`** by FTP to:

```text
/home/<account>/apps/hello-dotnet/
```

The main artifact is `app.dll`; keep the runtime files next to it.

## Panel configuration

Create it from **Applications → New application**:

| Field | Value |
| --- | --- |
| Name | `hello-dotnet` |
| Runtime | .NET |
| Version | Installed .NET version |
| Internal port | `8000` |
| Entrypoint | `app.dll` |
| Command | `dotnet` |
| Arguments | `app.dll,--urls,http://127.0.0.1:8000` |
| Start application | Enabled |

The URL argument binds ASP.NET to loopback and the selected application port.

## Publish it

Create a website with runtime **.NET**, select `hello-dotnet` as the application,
and use `public_html` as the document root. The application and the website
count as two resources toward `web_domains`.

See the [Python guide](python-app.md#troubleshooting) for the common 502 checks.
