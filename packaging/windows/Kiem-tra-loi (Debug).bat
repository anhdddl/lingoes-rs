@echo off
chcp 65001 > nul
title Kiem tra loi khoi dong Lingoes RS
echo ===================================================================
echo           BO CONG CU KIEM TRA KHOI DONG LINGOES RS
echo ===================================================================
echo.
echo [1/3] Kiem tra du lieu tu dien (data\lexiforge.db)...
if not exist "%~dp0data\lexiforge.db" (
    echo.
    echo ===============================================================
    echo [CANH BAO NGHIEP TRONG] KHONG TIM THAY TEP data\lexiforge.db !
    echo.
    echo Nguyen nhan pho bien:
    echo 1. Ban dang mo truc tiep file .bat hoac .exe tu trong file ZIP.
    echo    Windows se khong tu dong giai nen thu muc data.
    echo.
    echo Cach khac phuc:
    echo - Nhap chuot phai vao file LingoesRS-Windows-x64-portable.zip
    echo - Chon "Extract All..." (Giai nen tat ca)
    echo - Chon thu muc giai nen tren o C: hoac D:
    echo - Mo thu muc da giai nen roi moi chay file nay.
    echo ===============================================================
    echo.
    pause
    exit /b 1
) else (
    echo   + Da tim thay co so du lieu data\lexiforge.db OK!
)
echo.
echo [2/3] Dang khoi dong Lingoes RS...
echo (Neu co loi crash hoac loi man hinh, thong tin se hien ben duoi)
echo -------------------------------------------------------------------
"%~dp0lingoes-desktop.exe" --gui
set EXITCODE=%ERRORLEVEL%
echo -------------------------------------------------------------------
echo.
echo [3/3] Ket thuc tien trinh (Exit Code: %EXITCODE%)
echo.
if exist "%~dp0lingoes_startup.log" (
    echo --- NOI DUNG NHAT KY KHOI DONG (lingoes_startup.log) ---
    type "%~dp0lingoes_startup.log"
    echo -------------------------------------------------------------------
)
if exist "%~dp0lingoes_crash.log" (
    echo.
    echo --- CHI TIET LOI CRASH (lingoes_crash.log) ---
    type "%~dp0lingoes_crash.log"
    echo -------------------------------------------------------------------
)
echo.
echo Nhan phim bat ky de dong cua so nay...
pause > nul
