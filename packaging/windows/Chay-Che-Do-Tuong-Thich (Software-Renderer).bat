@echo off
title Lingoes RS (Che do tuong thich CPU Software)
echo Dang khoi dong Lingoes RS voi che do ve bang CPU (khong dung card man hinh/OpenGL)...
set SLINT_BACKEND=winit-software
start "" "%~dp0lingoes-desktop.exe" --software
