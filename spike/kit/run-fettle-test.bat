@echo off
rem Fettle test run: reads this PC, changes nothing, sends nothing.
rem No accented letters here: older consoles would garble them.
cd /d "%~dp0"
echo Fettle test run. It only reads this PC: it changes nothing and sends nothing.
echo Test di Fettle. Legge solo questo PC: non cambia nulla e non invia nulla.
echo.
set "LABEL="
set /p "LABEL=Short name for this PC / Nome breve per questo PC (e.g. old-laptop): "
if "%LABEL%"=="" set "LABEL=unlabelled"
rem fltmc only works as administrator and needs no service (net session fails when the Server service is off).
fltmc >nul 2>&1 && (set "MODE=admin") || (set "MODE=user")
echo.
echo [1/2] Raw test dump / Dati grezzi (%MODE%)...
"%~dp0fettle-spike.exe" --label "%LABEL%"
if not exist "%~dp0dumps" mkdir "%~dp0dumps"
echo [2/2] Fettle report (%MODE%)...
"%~dp0fettle-collect.exe" --out "%~dp0dumps\%LABEL%-%MODE%-report.json"
echo.
echo Done. Send everything in the "dumps" folder next to this file.
echo Fatto. Invia tutto quello che trovi nella cartella "dumps" accanto a questo file.
pause
