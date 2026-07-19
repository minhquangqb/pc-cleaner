# PC Cleaner

Phần mềm dọn dẹp máy tính cross-platform (macOS / Windows / Linux): quét junk/cache, tìm file lớn, tìm file trùng lặp. Mọi thao tác xóa đều chuyển vào Thùng rác.

## Stack

- **Tauri 2** — desktop shell, backend Rust trong `src-tauri/`
- **Vue 3 + TypeScript** (`<script setup>`) + **Tailwind CSS 4** (import qua `@tailwindcss/vite`, không có file tailwind.config)
- **pnpm** cho frontend, **cargo** cho backend

## Lệnh

```bash
pnpm tauri dev          # chạy app dev (user tự chạy — không tự động chạy)
pnpm build              # type-check (vue-tsc) + build frontend
pnpm tauri build        # đóng gói release
cd src-tauri && cargo check        # compile-check backend
cd src-tauri && cargo test --lib   # unit tests (safety module)
```

Lưu ý: `cargo` nằm ở `~/.cargo/bin` — nếu shell không thấy thì `export PATH="$HOME/.cargo/bin:$PATH"`.

## Kiến trúc

```
src/                          # Vue frontend
  App.vue                     # shell: sidebar 4 tab, KeepAlive views
  views/                      # DashboardView, JunkView, LargeFilesView, DupesView
  components/                 # ConfirmClean (modal xác nhận), ScanStatus (progress)
  composables/useScanProgress.ts  # lắng nghe event scan://progress theo task
  api.ts                      # wrapper invoke() + formatBytes
  types.ts                    # interface khớp với struct Serialize bên Rust
src-tauri/src/
  lib.rs                      # Tauri commands (async + spawn_blocking)
  safety.rs                   # validate_deletable + trash_paths — LỚP AN TOÀN DUY NHẤT
  junk.rs                     # quét 5 nhóm junk theo CategoryDef khai báo
  large.rs                    # quét file lớn (jwalk)
  dupes.rs                    # tìm trùng lặp 3 tầng: size → hash 64KB → full BLAKE3
  disk.rs                     # thông tin ổ đĩa (sysinfo)
  progress.rs                 # emit event scan://progress
```

Luồng dữ liệu: view gọi `api.ts` → `invoke()` command Rust → chạy trong `spawn_blocking` → trả JSON. Tiến trình quét đi ngược qua event `scan://progress` (payload `ScanProgress`, phân biệt bằng field `task`: `junk` | `large` | `dupes`).

## Quy tắc an toàn — KHÔNG ĐƯỢC VI PHẠM

Đây là app xóa file; một bug có thể mất dữ liệu người dùng:

- **Mọi lệnh xóa phải đi qua `safety::trash_paths`** — dùng `trash::delete` (vào Thùng rác), tuyệt đối không dùng `fs::remove_*` cho dữ liệu người dùng.
- `validate_deletable` bắt buộc trước khi xóa: chỉ cho phép path trong home hoặc temp dir, canonicalize để chặn `..`/symlink, từ chối protected paths (home root, Documents, Desktop, `.ssh`, `/System`, `C:\Windows`...).
- Thêm protected path mới vào `protected_roots()` khi thêm tính năng đụng vùng nhạy cảm; viết test cho mọi thay đổi trong `safety.rs`.
- UI luôn theo flow: quét → hiển thị → user chọn → modal ConfirmClean → mới xóa. Không bao giờ xóa tự động.
- Chỉ đưa vào danh sách junk những thứ tái tạo được (cache, temp, log). Không thêm thư mục chứa dữ liệu người dùng.

## Conventions

- UI strings bằng tiếng Việt (có dấu đầy đủ); code comments, tên biến/hàm bằng tiếng Anh.
- Thêm Tauri command mới: viết hàm trong module riêng → khai báo trong `lib.rs` → đăng ký vào `generate_handler!` → thêm wrapper trong `api.ts` + type trong `types.ts`.
- Struct trả về frontend dùng `#[derive(Serialize)]`, field snake_case (frontend types khớp snake_case, không đổi tên).
- Quét nặng dùng `jwalk`/`rayon` song song; emit progress có throttle (mỗi N item) để không spam event.
- Tauri plugin mới phải thêm permission vào `src-tauri/capabilities/default.json`.
