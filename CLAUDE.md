# mark-hulk

## Lokasi exe yang dipakai (user menyebutnya "unit test")

- Exe produksi ter-install di `C:\Users\Sarta\AppData\Local\mark-hulk\mark-hulk.exe`.
- Shortcut Desktop dan Start Menu "mark-hulk" menunjuk ke lokasi install tersebut, BUKAN ke `src-tauri\target\release`.

## Folder target

- `src-tauri\target` sudah dihapus (Juli 2026) dan aman dihapus lagi kapan saja setelah build.
- Setelah build rilis baru: perbarui exe di lokasi install AppData di atas (atau jalankan installer-nya lagi).
