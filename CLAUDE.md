# PC Cleaner

Phần mềm dọn dẹp máy tính cross-platform (macOS / Windows / Linux): quét junk/cache, tìm file lớn, tìm file trùng lặp, phân tích dung lượng thư mục theo cột (kiểu OmniDiskSweeper). Mọi thao tác xóa đều chuyển vào Thùng rác.

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
  App.vue                     # shell: sidebar 7 tab, KeepAlive views
  views/                      # DashboardView, JunkView, DevJunkView, UninstallView, LargeFilesView, TreeView, DupesView
  components/                 # ConfirmClean (modal xác nhận), ScanStatus (progress)
  composables/useScanProgress.ts  # lắng nghe event scan://progress theo task
  i18n/                       # vue-i18n: index.ts (setLocale + localStorage) + locales/vi.ts, en.ts
  api.ts                      # wrapper invoke() + formatBytes
  types.ts                    # interface khớp với struct Serialize bên Rust
src-tauri/src/
  lib.rs                      # Tauri commands (async + spawn_blocking), builder + window events
  safety.rs                   # validate_deletable + trash_paths — LỚP AN TOÀN DUY NHẤT
  junk.rs                     # quét 5 nhóm junk theo CategoryDef khai báo + junk_total_size (check nền)
  devjunk.rs                  # quét home tìm artifact dev theo project: node_modules / target / venv
  apps.rs                     # gỡ ứng dụng (macOS): liệt kê .app, đọc Info.plist, tìm leftover ~/Library
  tray.rs                     # tray icon + menu + watcher định kỳ (12h) notify khi rác > 5GB
  large.rs                    # quét file lớn (jwalk)
  tree.rs                     # phân tích dung lượng: Miller columns, size tính nền
  dupes.rs                    # tìm trùng lặp 3 tầng: size → hash 64KB → full BLAKE3
  disk.rs                     # thông tin ổ đĩa (sysinfo)
  progress.rs                 # emit event scan://progress
  i18n.rs                     # ngôn ngữ cho tray/notification (tr + persist file lang)
```

Luồng dữ liệu: view gọi `api.ts` → `invoke()` command Rust → chạy trong `spawn_blocking` → trả JSON. Tiến trình quét đi ngược qua event `scan://progress` (payload `ScanProgress`, phân biệt bằng field `task`: `junk` | `large` | `dupes` | `tree` | `dev` | `apps`).

Riêng tab Phân tích (`tree.rs` + `TreeView.vue`) chạy khác các tab quét-rồi-hiển-thị:

- Listing mỗi cột là `read_dir` trực tiếp (tức thì, không chờ quét); chỉ size thư mục lấy từ index nền.
- `start_tree_scan` spawn worker pool (tối đa 8 thread) walk toàn cây qua hàng đợi ưu tiên hot/cold, cộng dồn size file vào mọi ancestor trong `TreeState` (managed state, Arc + Mutex); scan mới hủy scan cũ qua generation counter; xong thì emit event `tree://done`.
- `set_tree_focus` (gọi khi user drill vào thư mục) đẩy các thư mục thuộc nhánh đang mở lên đầu hàng đợi (hot, LIFO/depth-first) — nhánh user đang xem được tính size trước, phần còn lại tính sau.
- Frontend poll refresh các cột đang mở mỗi 0.7s khi đang quét; size thư mục hiển thị hậu tố `+` khi chưa chốt.
- Sau khi xóa, `forget_tree_paths` trừ size khỏi index thay vì quét lại.

Tính size: dùng `scan::on_disk_size` (block thực trên đĩa, kiểu `du`) cho tab Phân tích và File lớn — cloud placeholder (iCloud / Google Drive chưa tải về) tính ~0, sparse/APFS-compressed tính đúng thực tế. Tab Trùng lặp giữ size logic vì so theo nội dung.

## Quy tắc an toàn — KHÔNG ĐƯỢC VI PHẠM

Đây là app xóa file; một bug có thể mất dữ liệu người dùng:

- **Mọi lệnh xóa phải đi qua `safety::trash_paths`** — dùng `trash::delete` (vào Thùng rác), tuyệt đối không dùng `fs::remove_*` cho dữ liệu người dùng.
- `validate_deletable` bắt buộc trước khi xóa: chỉ cho phép path trong home hoặc temp dir (ngoại lệ duy nhất: `.app` bundle nằm trực tiếp trong `/Applications` — phục vụ gỡ ứng dụng), canonicalize để chặn `..`/symlink, từ chối protected paths (home root, Documents, Desktop, `.ssh`, `/System`, `C:\Windows`...).
- Thêm protected path mới vào `protected_roots()` khi thêm tính năng đụng vùng nhạy cảm; viết test cho mọi thay đổi trong `safety.rs`.
- UI luôn theo flow: quét → hiển thị → user chọn → modal ConfirmClean → mới xóa. Không bao giờ xóa tự động.
- Chỉ đưa vào danh sách junk những thứ tái tạo được (cache, temp, log). Không thêm thư mục chứa dữ liệu người dùng.

## Conventions

- UI strings đi qua vue-i18n (`src/i18n/locales/vi.ts` + `en.ts`) — KHÔNG hard-code chuỗi hiển thị trong component; thêm string mới phải thêm cả hai locale (tiếng Việt có dấu đầy đủ). Switcher ở sidebar, lựa chọn lưu localStorage (`pc-cleaner-lang`) và sync sang Rust qua command `set_app_language` (tray menu + notification, persist vào app_config_dir/lang).
- Backend emit `phase` trong `scan://progress` là key ổn định (vd `walking`, `dupes_stage1`, `sizing:<category_id>`) — frontend dịch trong `ScanStatus.vue` (`scan.phases.*`); KHÔNG emit chuỗi ngôn ngữ tự nhiên từ Rust. Tên/mô tả nhóm junk và dev-artifact dịch theo `id`/`kind` (`junk.categories.*`, `dev.kinds.*`), backend chỉ gửi fallback.
- Code comments, tên biến/hàm bằng tiếng Anh.
- Icon UI dùng `@lucide/vue` (SVG, stroke theo `currentColor`) — KHÔNG dùng emoji làm icon. Icon inline cạnh text dùng class `inline size-4 align-[-2px]`; icon sidebar/trạng thái render qua `<component :is>`.
- Thêm Tauri command mới: viết hàm trong module riêng → khai báo trong `lib.rs` → đăng ký vào `generate_handler!` → thêm wrapper trong `api.ts` + type trong `types.ts`.
- Struct trả về frontend dùng `#[derive(Serialize)]`, field snake_case (frontend types khớp snake_case, không đổi tên).
- Quét nặng dùng `jwalk`/`rayon` song song; emit progress có throttle (mỗi N item) để không spam event.
- Tauri plugin mới phải thêm permission vào `src-tauri/capabilities/default.json`.
