@echo off
set PATH=C:\Users\MoZayed\.cargo\bin;C:\Users\MoZayed\.bun\bin;C:\Program Files\nodejs;C:\WINDOWS\system32;C:\WINDOWS;C:\Program Files\Git\cmd;%PATH%
cd /d F:\projects\NanoPrayer
bun run tauri:build
