# Changelog — Ứng Dụng Từ Bi

Tất cả thay đổi đáng chú ý của dự án sẽ được ghi lại trong file này.
Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
tuân thủ [Semantic Versioning](https://semver.org/lang/vi/).

---

## [0.5.0] — 2026-08-14 — Giai đoạn 5: Hạ tầng deploy (Docker + GitHub Actions + Coolify) + storage ảnh

### Thêm
- **Dockerfile multi-stage** với Rust 1.97.1, image final ~30 MB (glibc + stripped binary + static assets)
  - Stage 1 (builder): `rust:1.97.1-slim-bookworm` + pkg-config/libssl-dev/libpq-dev
  - Stage 2 (runtime): `debian:bookworm-slim` + libssl3/libpq5/ca-certificates/curl/tini
  - Run as non-root user `tubi` (UID 1001)
  - HEALTHCHECK bằng `curl /api/health`
  - ENTRYPOINT bằng `tini` để xử lý signals đúng (graceful shutdown)
- **`.dockerignore`** — loại bỏ target/, .git, .env, docs khỏi Docker context
- **`docker-compose.yml`** cho dev — Postgres 17-alpine + app cùng lúc
- **GitHub Actions workflow** `.github/workflows/docker.yml`:
  - Trigger: push tag `v*` hoặc push lên main (filter path)
  - Build & push Docker image lên `ghcr.io/mhieuhonda/ungdungtubi` với tags: version, `latest`, SHA
  - Cache build qua `cache-from/cache-to: type=gha`
  - Trigger Coolify webhook sau khi push xong (lấy URL từ secret `COOLIFY_WEBHOOK_URL`)
- **Migration 004**: bảng `images` (id, uploader_id, original_name, stored_filename, mime_type, size_bytes, sha256, width, height, purpose, is_public, created_at) + bảng `audit_log` (append-only, ghi lại mọi giao dịch quan trọng) + trigger `trigger_set_updated_at` tự cập nhật `updated_at` khi UPDATE users
- **API upload ảnh** `POST /api/upload-image` (form-data field `file`):
  - Giới hạn 5 MB/ảnh (configurable qua `MAX_UPLOAD_BYTES`)
  - Chỉ chấp nhận JPEG, PNG, WebP, GIF
  - Tính SHA-256 để chống trùng lặp (cùng user upload lại ảnh cũ → trả về URL cũ)
  - Parse width/height từ header ảnh (PNG/JPEG/GIF/WebP)
  - Lưu file vào `upload_dir` với tên `<uuid>.<ext>`
  - Trả về JSON `{ id, url, size, mime_type, width, height, sha256 }`
- **`GET /api/upload-info`** — trả về giới hạn upload + danh sách MIME types cho phép
- **Auto-run migrations** khi khởi động (set `RUN_MIGRATIONS=true` hoặc `APP_ENV=production` tự động bật)
- **Health check cải tiến** — giờ check cả DB (`SELECT version()`) và trả về `database.status` + `database.version`
- **Graceful shutdown** — `shutdown_timeout(30s)`, 4 workers
- **DB pool size tunable** qua env `DB_MAX_CONNECTIONS` (mặc định 10)
- **Static dir + upload dir configurable** qua env `STATIC_DIR` / `UPLOAD_DIR` / `UPLOAD_URL_PREFIX` (cho Docker volume mapping)
- **Release profile tối ưu**: `opt-level=3`, `lto="thin"`, `codegen-units=1`, `strip="symbols"`, `panic="abort"` (binary nhỏ + chạy nhanh)

### Sửa
- **[SECURITY] Bỏ GET `/dang-xuat`** — chỉ còn POST để chống CSRF (kẻ tấn công không thể nhúng `<img src="/dang-xuat">` để log user ra ngoài)
- **[SECURITY] Logout form dùng JavaScript submit** thay vì link GET trong `layout.html` và `placeholder_page()` — giữ UX click như cũ nhưng an toàn
- **[BUG] `static_dir` dùng relative path** `src/static` — khi chạy trong Docker với working directory khác, không tìm thấy static files. Nay lấy từ env `STATIC_DIR` (Docker set `/app/static`)
- **[BUG] `USER_COLUMNS` thiếu `avatar_upload_id`** — sau migration 004 thêm cột này, `SELECT ... RETURNING` sẽ fail nếu không liệt kê. Đã thêm vào cả `handlers::USER_COLUMNS` và `handlers::auth::USER_COLUMNS`
- **[BUG] `render_user_menu_html` dùng `format!` với `'` trong string** — Rust 2024 format macro strict về `'` trong string literals. Đổi sang `push_str` để tránh vấn đề này
- **[CLEANUP] `fs::Files::new(...).disable_listing()` không tồn tại** — actix-files mặc định đã không list thư mục. Bỏ method không tồn tại này
- **[CLEANUP] `let _ = u` trong `render_mobile_user_menu_html`** — clippy warn, đã bỏ

### Đổi
- **`Cargo.toml`**:
  - version `0.4.0` → `0.5.0`
  - Thêm dependencies: `actix-multipart` (upload), `sha2` (checksum), `bytes` (stream), `mime` (validation), `futures-util` (stream)
  - Thêm `[profile.release]` tối ưu
- **`src/config.rs`**:
  - Thêm fields: `static_dir`, `upload_dir`, `max_upload_bytes`, `db_max_connections`, `upload_url_prefix`
  - Tất cả đều lấy từ env với default hợp lý cho cả dev và Docker
- **`src/main.rs`**:
  - Auto-migrate với `sqlx::migrate!`
  - DB ping khi khởi động (log PostgreSQL version)
  - Tạo `upload_dir` nếu chưa tồn tại
  - Đăng ký routes `/api/upload-info`, `/api/upload-image`
  - Health check giờ nhận `pool` và check DB
  - `workers(4)`, `shutdown_timeout(30)`
- **`src/handlers/mod.rs`**:
  - Export `pub const USER_COLUMNS` và `pub async fn get_user_from_session` cho handler uploads dùng
  - Thêm `pub mod uploads;`
- **`src/handlers/auth.rs`**: đồng bộ `USER_COLUMNS` (thêm `avatar_upload_id`)
- **`src/models/user.rs`**: thêm field `avatar_upload_id: Option<Uuid>`
- **`templates/layout.html`**: logout dùng JavaScript submit thay vì GET link
- **`.env.example`**: thêm các biến mới (`STATIC_DIR`, `UPLOAD_DIR`, `UPLOAD_URL_PREFIX`, `MAX_UPLOAD_BYTES`, `DB_MAX_CONNECTIONS`, `RUN_MIGRATIONS`)
- **`README.md`**: cập nhật cho v0.5 — thêm Docker + GitHub Actions + Coolify workflow
- **`CHANGELOG.md`**: thêm mục v0.5

### Bump version
- `Cargo.toml`: 0.4.0 → 0.5.0
- `main.rs` health endpoint: 0.4.0 → 0.5.0, thêm `phase: 5` + `phase_name` + `database` object
- `templates/layout.html` footer: v0.4 → v0.5

---

## [0.4.0] — 2026-08-13 — Giai đoạn 4: Hồ sơ thành viên & Hệ thống cấp bậc

### Thêm
- **Trang hồ sơ cá nhân** `/ca-nhan` với:
  - Avatar từ Google (hoặc chữ cái đầu tên nếu không có)
  - Hiển thị cấp bậc, màu sắc, icon riêng cho từng cấp
  - Thống kê Niệm Lực A, Tiền K, giới tính
  - Pháp danh, pháp hiệu, bút danh, tiểu sử
- **Form chỉnh sửa hồ sơ** với các trường:
  - Tên hiển thị (bắt buộc, tối đa 100 ký tự)
  - Pháp danh (tùy chọn)
  - Pháp hiệu (tùy chọn)
  - Bút danh (tùy chọn)
  - Giới tính (Nam/Nữ/Khác)
  - Tiểu sử (tùy chọn, tối đa 500 ký tự)
- **Endpoint POST `/ca-nhan/cap-nhat`** — cập nhật hồ sơ an toàn:
  - Validate input (tên không rỗng, gender thuộc enum)
  - Không cho phép sửa email, rank, số dư A/K, is_active
  - Chuẩn hoá optional fields (None nếu rỗng)
- **Hệ thống 9 cấp bậc** với bảng `member_ranks`:
  - 🌱 Người Mới (0 K) → 👑 Đại Gia (100.000 K)
  - Mỗi cấp bậc có: tên, mô tả, màu hex, emoji, min_k_balance
  - Hiển thị tiến độ cấp bậc trên trang profile
- **Migration 003**: thêm cột `phap_danh`, `phap_hieu`, `but_danh`, `gender`, `bio` vào `users`; tạo bảng `member_ranks` + seed 9 cấp bậc mặc định
- **Hiển thị icon + tên cấp bậc** trên header (gần tên user, có tooltip)
- **Model `MemberRank`** + helper `ProfileUpdate` cho form data
- **Helper methods trên User**:
  - `rank_display()` — tên tiếng Việt cấp bậc
  - `rank_icon()` — emoji tương ứng
  - `rank_color()` — mã màu hex
  - `display_label()` — tên hiển thị ưu tiên theo pháp danh > pháp hiệu > bút danh > display_name
  - `gender_display()` — Nam/Nữ/Khác
- **Document `USER_COLUMNS`** — danh sách cột cố định để tránh drift giữa các SQL queries

### Sửa
- **[BUG] Login button trong `login.html` trỏ sai endpoint** — trước đây trỏ về `/dang-nhap` (GET render lại template), nay trỏ đúng `/auth/google` để trigger OAuth flow
- **[BUG] Mobile menu trong `layout.html` thiếu nút "Hồ sơ/Thoát" khi đã đăng nhập** — nay hiển thị đầy đủ cả 2 trạng thái
- **[CLEANUP] Migration 002 có chữ Hán "映射" sót lại** — đổi thành "ánh xạ" tiếng Việt
- **[DRY] SQL queries trong `auth.rs` và `handlers/mod.rs`** dùng chung `USER_COLUMNS` constant thay vì lặp lại danh sách cột
- **[SECURITY] Logout endpoint** — đã có POST (chuẩn CSRF-safe), giữ GET cho link đơn giản
- **[CODE] Loại bỏ dependency `actix-rt` không dùng trực tiếp** (actix-web đã re-export)

### Bump version
- `Cargo.toml`: 0.3.0 → 0.4.0
- `main.rs` health endpoint: 0.3.0 → 0.4.0, thêm `phase: 4` + `phase_name`
- `README.md`, `templates/layout.html`, `.env.example`: cập nhật nhãn v0.4

---

## [0.3.0] — Giai đoạn 3: Chuyển sang Google OAuth

### Thêm
- Tích hợp Google OAuth 2.0 (Authorization Code Flow)
- Endpoint `/auth/google` + `/auth/google/callback`
- Migration 002: `password_hash` NULL được, thêm `google_sub`, `avatar_url`, `email_verified`
- State chống CSRF (cookie HttpOnly, TTL 10 phút)
- Auto-link tài khoản Google với tài khoản cũ (theo email)
- Bỏ hoàn toàn form đăng ký email/password

---

## [0.2.0] — Giai đoạn 2: Hệ thống xác thực email/password

### Thêm
- Form đăng ký thành viên (email, mật khẩu, tên hiển thị)
- Đăng nhập (email + password)
- Session management (cookie-based, SQLx session store)
- Logout & bảo vệ route
- Migration 001: bảng `users`, `sessions`

---

## [0.1.0] — Giai đoạn 1: Nền móng hạ tầng cốt lõi

### Thêm
- Khởi tạo project Rust (Actix-web + Askama + SQLx + PostgreSQL)
- Cấu hình HTMX + Alpine.js + Tailwind CSS
- Trang landing page / trang chủ
- Hệ thống template layout (header, footer, navigation)
- Cấu hình domain `tubi.louis.vangioitutien.com`

---

*Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.* 🪷
