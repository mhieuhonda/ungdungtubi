# Changelog — Ứng Dụng Từ Bi

Tất cả thay đổi đáng chú ý của dự án sẽ được ghi lại trong file này.
Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
tuân thủ [Semantic Versioning](https://semver.org/lang/vi/).

---

## [0.6.0] — 2026-08-14 — Giai đoạn 6: Cộng Đồng Foundation (Nhóm + Chủ Đề + Bình luận)

### Thêm
- **Chuyên mục Cộng Đồng chính thức ra mắt** — `/cong-dong` giờ là trang thật (không còn placeholder).
  - Trang chính hiển thị danh sách nhóm công khai + chủ đề mới nhất (Lướt Nhóm / Lướt Chủ Đề tabs)
  - Hero banner với nút "Tạo Nhóm Mới" (auth) hoặc "Đăng nhập để tham gia" (guest)
- **Hệ thống Nhóm (Groups)** — đơn vị tổ chức cộng đồng:
  - Tạo nhóm: tên (bắt buộc), mô tả, phân loại, visibility (public/private/hidden), require_approval
  - Trang nhóm `/cong-dong/nhom/{slug}` hiển thị thông tin + danh sách chủ đề
  - Tham gia / rời nhóm (POST-only để chống CSRF)
  - Slug tự sinh từ tên (loại bỏ dấu tiếng Việt, collapse dashes)
  - Owner tự động trở thành `group_members.role = 'owner'`
  - Owner không thể rời nhóm (phải chuyển quyền trước)
- **Hệ thống Chủ Đề (Topics)** — bài viết trong nhóm (diễn đàn):
  - Tạo chủ đề: title (bắt buộc, max 200 ký tự), body (bắt buộc, hỗ trợ multiline)
  - Chỉ thành viên active của nhóm mới được tạo chủ đề
  - Chủ đề hiển thị: tiêu đề, body, author (avatar + tên + cấp bậc), thời gian tương đối
  - Hỗ trợ ghim (`is_pinned`) và khoá (`is_locked`) — schema sẵn sàng, UI ở giai đoạn sau
  - Tự tăng `view_count` mỗi lần xem trang chủ đề
- **Hệ thống Bình luận (Comments)** — bình luận trên chủ đề:
  - Form bình luận nhanh ngay trên trang chủ đề
  - Hỗ trợ reply (parent_id) — schema sẵn sàng, UI nested ở giai đoạn sau
  - Validate body (không rỗng, tối đa 5000 ký tự)
  - Không bình luận được nếu chủ đề bị khoá (`is_locked`)
- **Phân loại nhóm (group_categories)** — 9 phân loại mặc định:
  - Tu Học · Niệm Phật · Kinh Sách · Thiền Định · Pháp Thoại · Chia Sẻ · Thiện Nguyện · Âm Nhạc · Khác
- **Migration 005**: 5 bảng mới + 4 triggers + 9 seed categories
  - `group_categories` (slug, name, icon, sort_order)
  - `groups` (id, slug, name, description, category_id, owner_id, cover_upload_id, visibility, require_approval, member_count, topic_count, is_active, timestamps)
  - `group_members` (group_id, user_id, role, status, joined_at) — UNIQUE(group_id, user_id)
  - `topics` (id, group_id, author_id, title, body, is_pinned, is_locked, comment_count, view_count, is_active, timestamps)
  - `comments` (id, topic_id, author_id, parent_id, body, is_active, timestamps)
  - Trigger `trg_*_set_updated_at` — tự cập nhật `updated_at` cho groups/topics/comments
  - Trigger `trg_*_count` — tự cập nhật `member_count`, `topic_count`, `comment_count` khi INSERT/DELETE
- **Models `community.rs`** — 8 struct:
  - `Group`, `GroupWithCategory`, `Topic`, `TopicWithAuthor`, `Comment`, `CommentWithAuthor`, `GroupMember`, `GroupCategory`
  - Form structs: `GroupCreateForm`, `TopicCreateForm`, `CommentCreateForm`
  - Helper methods: `visibility_display()`, `visibility_icon()`, `category_icon_or_lotus()`, `category_name_or_other()`, `time_ago()`, `body_excerpt()`, `author_initial()`, `role_display()`, `role_icon()`, `is_staff()`
- **Handlers `community.rs`** — 10 endpoint:
  - `GET /cong-dong` — trang chính (list groups + hot topics)
  - `GET /cong-dong/tao-nhom` — form tạo nhóm (auth)
  - `POST /cong-dong/tao-nhom` — tạo nhóm mới (auth, transaction)
  - `GET /cong-dong/nhom/{slug}` — xem nhóm + danh sách chủ đề
  - `POST /cong-dong/nhom/{slug}/tham-gia` — tham gia nhóm (auth, ON CONFLICT DO NOTHING)
  - `POST /cong-dong/nhom/{slug}/roi-khoi` — rời nhóm (auth, owner không được rời)
  - `GET /cong-dong/nhom/{slug}/tao-chu-de` — form tạo chủ đề (auth + member)
  - `POST /cong-dong/nhom/{slug}/tao-chu-de` — tạo chủ đề (auth + member)
  - `GET /cong-dong/chu-de/{id}` — xem chủ đề + bình luận
  - `POST /cong-dong/chu-de/{id}/binh-luan` — đăng bình luận (auth)
- **Templates `templates/community/`** — 5 template Askama:
  - `index.html` — trang chính với tabs Lướt Nhóm / Lướt Chủ Đề
  - `group.html` — trang nhóm với topic list + action buttons (tham gia/rời/tạo chủ đề)
  - `topic.html` — trang chủ đề với comment form + comment list
  - `create_group.html` — form tạo nhóm với visibility radio + category select
  - `create_topic.html` — form tạo chủ đề
- **Helper `time_ago_display()`** — format thời gian tương đối tiếng Việt ("vừa xong", "5 phút trước", "2 ngày trước", ...)
- **Helper `slugify()`** — tạo slug từ tên tiếng Việt (loại bỏ dấu, thay whitespace bằng `-`)
- **Helper `ensure_unique_slug()`** — thêm hậu tố UUID 6 ký tự nếu slug trùng
- **Helper `fetch_categories()`** — lấy danh sách category theo sort_order
- **Helper `get_membership()`** — kiểm tra user có phải thành viên nhóm không

### Sửa
- **[REFACTOR] Handler `cong_dong` trong `handlers/mod.rs`** — không còn dùng `placeholder_page`. Nay delegate cho `community::cong_dong_index`.
- **[DRY] `placeholder_page`** vẫn được dùng cho các chuyên mục chưa phát triển (Không Gian, Bạn Bè, Kinh Sách, Quỹ Từ Bi, Thương Thành, Bảng Xếp Hạng).

### Đổi
- **`Cargo.toml`**: version `0.5.0` → `0.6.0`
- **`main.rs`**:
  - Log khởi động: v0.5 → v0.6, đổi phase_name thành "Cộng Đồng Foundation"
  - Health endpoint: `version: 0.6.0`, `phase: 6`, `phase_name: "Cộng Đồng Foundation — Nhóm + Chủ Đề + Bình luận"`
  - Thêm 8 routes mới cho Cộng Đồng
- **`templates/layout.html`**: footer v0.5 → v0.6
- **`src/handlers/mod.rs`**: footer placeholder_page v0.5 → v0.6
- **`src/models/mod.rs`**: export thêm `community` module
- **`src/handlers/mod.rs`**: export thêm `community` module

### Bump version
- `Cargo.toml`: 0.5.0 → 0.6.0
- `main.rs` health endpoint: 0.5.0 → 0.6.0, phase 5 → 6
- `templates/layout.html` footer: v0.5 → v0.6
- `README.md`: cập nhật routes + version v0.6

### Lộ trình tiếp theo (v0.7+)
- **v0.7**: Live Chat thời gian thực (WebSocket) trong nhóm
- **v0.8**: Ghim/khoá chủ đề, sticky topics, pagination
- **v0.9**: Nested comments (reply tree), vote/like chủ đề và bình luận
- **v0.10**: Quỹ Từ Bi + Thương Thành
- **v0.12**: Cộng Đồng hoàn thiện (theo placeholder): pagination, search, filter, mod tools

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
