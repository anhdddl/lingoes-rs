LINGOES RS 0.1.2 - BAN WINDOWS 11 / WINDOWS 10 X64 PORTABLE
=============================================================

1. TUONG THICH:
- Windows 11 (64-bit) - Ho tro toi uu man hinh High-DPI (PerMonitorV2 Scaling 125%, 150%, 200%), icon he thong Aero 3D native.
- Windows 10 (64-bit) / Windows Server.
- Khong yeu cau cai dat Microsoft Visual C++ Redistributable (CRT da duoc lien ket tinh +crt-static).

2. HUONG DAN CHAY (QUAN TRONG):
- BUOC 1: GIAI NEN TOAN BO TEP ZIP (Extract All).
  * Chuot phai vao file ZIP -> Chon "Extract All..." (Giai nen tat ca).
  * Chon thu muc giai nen tren o dia (vi du: C:\LingoesRS hoac D:\LingoesRS).
  * LUU Y NGHIEP TRONG: Khong mo file .exe truc tiep ben trong file ZIP vi Windows se khong doc duoc thu muc du lieu "data\lexiforge.db".
- BUOC 2: Chay file "lingoes-desktop.exe" hoac "Chay-Lingoes.bat".

3. HUONG DAN FIX BUG / XU LY KHI KHONG MO DUOC TREN WINDOWS 11:
- Truong hop A: Windows SmartScreen chan ("Windows protected your PC" / "Windows da bao ve PC cua ban"):
  * Nhap vao dong chu "More info" (Thong tin khac) -> Chon nut "Run anyway" (Van chay).
  * Hoac chuot phai vao file lingoes-desktop.exe -> Chon Properties -> Tick vao o "Unblock" (Bo chan) o duoi cung -> Bam OK.
- Truong hop B: May khong co card do hoa hoac loi trinh dieu khien OpenGL:
  * Nhap dup vao file "Chay-Che-Do-Tuong-Thich (Software-Renderer).bat" de chay bang che do ve CPU.
- Truong hop C: Muon xem chi tiet bao loi he thong:
  * Nhap dup vao file "Kiem-tra-loi (Debug).bat". Cua so dong lenh se hien thi chi tiet tung buoc khoi dong va bat ky ma loi nao.
  * Hoac mo file nhat ky "lingoes_startup.log" va "lingoes_crash.log" tu dong sinh ra trong thu muc.
- Truong hop D: Tien trinh cu bi treo ngam trong Task Manager:
  * Mo Task Manager (Ctrl + Shift + Esc), tim "lingoes-desktop.exe" va bam End Task, sau do mo lai.

4. DU LIEU TU DIEN DI KEM (Tong cong 403.887 muc tu):
- Tu dien Anh - Viet tong hop: 228.613 muc tu.
- Tu dien Viet - Anh (OVDP - Ho Ngoc Duc): 42.103 muc tu.
- Tu dien Viet - Anh (SPDict): 133.171 muc tu.
- Bang 359 dong tu bat quy tac, Bang quy doi don vi, Bang IPA, Bang ma vung dien thoai.
- Database nam tai data\lexiforge.db.
- Che do portable hoan toan: moi cau hinh va lich su duoc luu tai thu muc data.
