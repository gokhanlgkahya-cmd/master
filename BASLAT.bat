@echo off
echo ================================================
echo  RiskPlatform v3.2 - Kurulum ve Baslangic
echo ================================================

REM package.json var mi kontrol et
if not exist "package.json" (
  echo HATA: Bu dosyayi trading-bot klasorune koyun!
  pause
  exit
)

echo.
echo [1/3] Node moduller yukleniyor...
call npm install

echo.
echo [2/3] Python bagimliliklari kontrol ediliyor...
python -m pip install flask flask-cors --break-system-packages --quiet

echo.
echo [3/3] Baslatiliyor...
echo.
echo  Python API : http://localhost:5000
echo  React App  : http://localhost:3000
echo.

REM Python API'yi arka planda baslat
start "RiskPlatform Python API" cmd /k "python api_server.py"

REM 3 saniye bekle API ayaga kalksin
timeout /t 3 /nobreak >nul

REM React'i baslat
call npm run dev

pause
