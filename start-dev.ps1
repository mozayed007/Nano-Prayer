$pathToAdd = "C:\Users\MoZayed\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;C:\Users\MoZayed\.cargo\bin"
if (-not $env:Path.Contains("stable-x86_64-pc-windows-msvc")) {
    $env:Path = "$pathToAdd;" + $env:Path
}
npm run tauri:dev