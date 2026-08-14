# 🪷 Ứng Dụng Từ Bi

> *Siêu thoát không siêu thích. Giải thoát không giải thích. Buông bỏ mới có thể trở về.*

**Domain:** [tubi.louis.vangioitutien.com](https://tubi.louis.vangioitutien.com)

## Tầm Nhìn

Xây dựng một hệ sinh thái giúp mọi người có thể ứng dụng Từ Bi vào cuộc sống, tu học và giải trí, từ đó hiểu rõ hơn về bản chất của khổ đau, giác ngộ và giải thoát.

**Triết lý cốt lõi:** Tu cũng niệm Phật. Chơi cũng niệm Phật.

## Công Nghệ

| Thành phần | Công nghệ |
|-----------|-----------|
| Backend | Rust 1.97.1 + Axum 0.8 |
| Template | Askama 0.14 (type-safe HTML templates) |
| Database | PostgreSQL 17 + SQLx (async, compile-time checked) |
| Frontend | HTMX (server-driven UI) + Alpine.js (reactive) |
| Styling | Tailwind CSS |
| Auth | Google OAuth 2.0 (OpenID Connect — userinfo) — đăng nhập duy nhất |
| Container | Docker (multi-stage build, ~30 MB image) |
| CI/CD | GitHub Actions → GHCR → Coolify webhook → auto deploy |

## 4 Chuyên Mục Chính

1. 🌍 **Không Gian** – Không gian cá nhân, cộng tu, niệm Phật
2. 👥 **Cộng Đồng** – Diễn đàn, nhóm, chủ đề, live chat
3. 👤 **Bạn Bè** – Kết nối, nhắn tin, gửi thư
4. 📚 **Kinh Sách** – Thư viện kinh sách Phật giáo & Đạo giáo

---

## Lộ Trình 25 Giai Đoạn Phát Triển

### Giai đoạn 1: Kiến tạo nền móng — Thiết lập dự án & hạ tầng cốt lõi ✅ (v0.1)
- Khởi tạo project Rust (Axum + Askama + SQLx + PostgreSQL)
- Cấu hình HTMX + Alpine.js + Tailwind CSS
- Thiết kế database schema nền tảng (users, sessions)
- Trang landing page / trang chủ
- Hệ thống template layout (header, footer, navigation)
- Cấu hình domain `tubi.louis.vangioitutien.com`
- **Mục tiêu:** Server chạy được, hiển thị trang chủ với giao diện cơ bản

### Giai đoạn 2: Hệ thống xác thực — Đăng ký & Đăng nhập ✅ (v0.2)
- Form đăng ký thành viên (email, mật khẩu, tên hiển thị)
- Đăng nhập (email + password)
- Session management (cookie-based, SQLx session store)
- Logout & bảo vệ route (xoá session khỏi database)
- Xác thực người dùng từ session cookie trên mọi trang
- Kiểm tra `is_active` khi đăng nhập (chặn tài khoản bị vô hiệu)
- Migrate database: bảng `users`, `sessions`
- **Mục tiêu:** Thành viên có thể đăng ký, đăng nhập, đăng xuất

### Giai đoạn 3: Chuyển sang Google OAuth — Đăng nhập duy nhất bằng Google ✅ (v0.3)
- Tích hợp Google OAuth 2.0 (Authorization Code Flow)
- **Bỏ hoàn toàn form đăng ký email/password** — web chỉ còn đăng nhập/đăng ký bằng Google
- Endpoint `/auth/google` → chuyển hướng sang Google consent
- Endpoint `/auth/google/callback` → đổi code lấy access_token, gọi userinfo, upsert user
- Tự động link tài khoản Google với tài khoản cũ (nếu email trùng nhau)
- State chống CSRF (cookie HttpOnly, SameSite=Lax, TTL 10 phút)
- Cookie session_id bảo mật (HttpOnly, SameSite=Lax, Secure khi production)
- Migration 002: `password_hash` NULL được, thêm `google_sub`, `avatar_url`, `email_verified`
- Trang `/dang-nhap` chỉ còn nút "Đăng nhập bằng Google"
- **Mục tiêu:** Người dùng chỉ đăng nhập qua Google; tài khoản cũ vẫn dùng được

### Giai đoạn 4: Hồ sơ thành viên & Hệ thống cấp bậc ✅ (v0.4)
- Trang hồ sơ cá nhân `/ca-nhan` với avatar, cấp bậc, thống kê A/K
- Form chỉnh sửa hồ sơ: tên hiển thị, pháp danh, pháp hiệu, bút danh, giới tính, tiểu sử
- Hệ thống cấp bậc 9 mức: 🌱 Người Mới → 👑 Đại Gia
  | # | Cấp bậc | Màu | Icon | Min K |
  |---|---------|-----|------|-------|
  | 1 | Người Mới | #9E9E9E | 🌱 | 0 |
  | 2 | Người Thường | #795548 | 🍃 | 1 |
  | 3 | Người Bình Thường | #558B2F | 🌿 | 10 |
  | 4 | Người Tốt | #388E3C | 🌳 | 100 |
  | 5 | Người Khá Tốt | #2E7D32 | 🌲 | 500 |
  | 6 | Người Rất Tốt | #1B5E20 | 🎋 | 1.000 |
  | 7 | Người Cực Kỳ Tốt | #00695C | 🏆 | 5.000 |
  | 8 | Thiện Nhân | #FFB300 | 🪷 | 10.000 |
  | 9 | Đại Gia | #FF6F00 | 👑 | 100.000 |
- Hiển thị icon + tên cấp bậc trên header (gần tên user)
- Hiển thị cấp bậc hiện tại + tiến độ各级 bậc trên profile
- Migration 003: thêm cột `phap_danh`, `phap_hieu`, `but_danh`, `gender`, `bio` vào `users`; tạo bảng `member_ranks` + seed 9 cấp bậc mặc định
- Endpoint POST `/ca-nhan/cap-nhat` để cập nhật hồ sơ (validate input, không cho sửa email/rank/số dư)
- **Mục tiêu:** Hồ sơ hoạt động, cấp bậc hiển thị đúng

### Giai đoạn 5: Hạ tầng deploy — Docker + Coolify + storage ảnh ✅ (v0.5)
- **Dockerfile multi-stage** với Rust 1.97.1, image final ~30 MB (glibc + stripped binary)
- **GitHub Actions** workflow: build → push Docker image lên GHCR → trigger Coolify webhook
- **Coolify** auto pull image mới + deploy lên domain `tubi.louis.vangioitutien.com`
- **PostgreSQL 17** trên sub VPS (10.187.247.3) làm database + storage
- **API upload ảnh** `/api/upload-image` (max 5 MB/ảnh, JPEG/PNG/WebP/GIF)
- Migration 004: bảng `images` + `audit_log` + trigger `updated_at` tự động
- Auto-run migrations khi khởi động (set `RUN_MIGRATIONS=true`)
- Health check endpoint `/api/health` giờ check cả DB
- **[SECURITY]** Bỏ GET `/dang-xuat` để chống CSRF — chỉ còn POST
- **[SECURITY]** Logout form dùng JavaScript submit thay vì link GET
- Graceful shutdown (30s timeout), 4 workers
- DB pool size tunable qua env `DB_MAX_CONNECTIONS`
- Release profile tối ưu (LTO thin, strip symbols, panic=abort)
- **Mục tiêu:** Web chạy production ổn định, deploy tự động, sẵn sàng cho giai đoạn 6+

### Giai đoạn 6: Cộng Đồng Foundation — Nhóm + Chủ Đề + Bình luận ✅ (v0.6)
- **Chuyên mục Cộng Đồng chính thức ra mắt** — không còn placeholder
- **Hệ thống Nhóm (Groups)**:
  - Tạo nhóm với tên, mô tả, phân loại (9 categories), visibility (public/private/hidden), require_approval
  - Trang nhóm `/cong-dong/nhom/{slug}` hiển thị thông tin + danh sách chủ đề
  - Tham gia / rời nhóm (POST-only, chống CSRF)
  - Slug tự sinh từ tên, đảm bảo duy nhất (thêm hậu tố UUID nếu trùng)
- **Hệ thống Chủ Đề (Topics)** — bài viết trong nhóm (diễn đàn):
  - Tạo chủ đề với title (max 200) + body (multiline)
  - Chỉ thành viên active mới được tạo chủ đề
  - Hỗ trợ ghim (`is_pinned`) + khoá (`is_locked`) ở schema, UI giai đoạn sau
  - Tự tăng `view_count` khi xem chủ đề
- **Hệ thống Bình luận (Comments)** — bình luận trên chủ đề:
  - Form bình luận nhanh ngay trên trang chủ đề
  - Hỗ trợ reply (parent_id) ở schema, UI nested giai đoạn sau
  - Validate body (không rỗng, tối đa 5000 ký tự)
  - Không bình luận được nếu chủ đề bị khoá
- **Migration 005**: 5 bảng mới (group_categories, groups, group_members, topics, comments) + 4 triggers + 9 seed categories
- **Giao diện Lướt**: tabs "Lướt Nhóm" / "Lướt Chủ Đề" trên trang chính Cộng Đồng
- **10 endpoint** mới cho Cộng Đồng (xem routes bên dưới)
- **Mục tiêu:** Cộng Đồng hoạt động đầy đủ — tạo nhóm, tham gia, tạo chủ đề, bình luận

### Giai đoạn 7–25: *(xem kế hoạch chi tiết trong HieuLouis/)*

---

## Cấu Trúc Dự Án (Giai đoạn 6 / v0.6)

```
ungdungtubi/
├── src/
│   ├── main.rs              # Entry point + routes + auto-migrate + health check DB
│   ├── config.rs            # Config (DB, host, Google OAuth, static_dir, upload_dir)
│   ├── db/
│   │   └── mod.rs           # Database helpers (session cleanup)
│   ├── errors/
│   │   └── mod.rs           # AppError enum with HTTP response mapping
│   ├── handlers/
│   │   ├── mod.rs           # Page handlers + session auth + profile update
│   │   ├── auth.rs          # google_login, google_callback, logout (POST-only)
│   │   ├── community.rs     # [v0.6] Groups + Topics + Comments handlers (10 endpoints)
│   │   └── uploads.rs       # [v0.5] Upload ảnh API (5MB max, SHA-256, dimensions)
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs          # User, GoogleUserInfo, MemberRank, ProfileUpdate
│   │   └── community.rs     # [v0.6] Group, Topic, Comment, GroupMember, GroupCategory
│   └── static/
│       ├── css/app.css
│       ├── js/app.js
│       └── uploads/         # [v0.5] Nơi lưu ảnh user upload
├── templates/                # Askama templates (Vietnamese)
│   ├── layout.html
│   ├── home.html
│   ├── profile.html
│   ├── auth/
│   │   └── login.html
│   └── community/            # [v0.6]
│       ├── index.html        # Trang chính Cộng Đồng (Lướt Nhóm / Lướt Chủ Đề)
│       ├── group.html        # Trang nhóm + topic list
│       ├── topic.html        # Trang chủ đề + bình luận
│       ├── create_group.html # Form tạo nhóm
│       └── create_topic.html # Form tạo chủ đề
├── migrations/
│   ├── 001_create_users_sessions.sql
│   ├── 002_google_oauth.sql
│   ├── 003_member_profile_ranks.sql
│   ├── 004_storage_images_audit.sql  # [v0.5] images + audit_log + trigger updated_at
│   └── 005_community_groups_topics_comments.sql  # [v0.6] groups + group_members + topics + comments + triggers
├── .github/workflows/
│   └── docker.yml            # [v0.5] Build + push + trigger Coolify
├── HieuLouis/                # Tài liệu dự án
├── Cargo.toml                # v0.6.0, Rust 1.97, release profile tối ưu
├── Dockerfile                # [v0.5] Multi-stage Rust 1.97.1, ~30 MB
├── docker-compose.yml        # [v0.5] Dev environment (Postgres 17 + app)
├── .env.example              # Template cấu hình môi trường v0.5
└── README.md
```

## Cài Đặt & Chạy

### Local dev với Docker Compose (Khuyên dùng)

```bash
# 1. Clone
git clone https://github.com/mhieuhonda/ungdungtubi.git
cd ungdungtubi

# 2. Tạo .env từ .env.example
cp .env.example .env
# Điền GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET (lấy từ Google Cloud Console)
# Đặt GOOGLE_REDIRECT_URI=http://localhost:8080/auth/google/callback

# 3. Chạy với Docker Compose (Postgres 17 + app cùng lúc)
docker compose up -d

# Server: http://localhost:8080
# DB: postgres://tubi:tubi_password@localhost:5432/ungdungtubi
```

### Local dev thủ công (không Docker)

```bash
# 1. Clone
git clone https://github.com/mhieuhonda/ungdungtubi.git
cd ungdungtubi

# 2. Tạo .env
cp .env.example .env
# Điền DATABASE_URL + GOOGLE_CLIENT_ID + GOOGLE_CLIENT_SECRET

# 3. Cấu hình Google OAuth
#    - Vào Google Cloud Console → APIs & Services → Credentials
#    - Tạo OAuth 2.0 Client ID (Web application)
#    - Thêm Authorized redirect URIs khớp với GOOGLE_REDIRECT_URI

# 4. Tạo database + chạy migrations
createdb ungdungtubi
psql -d ungdungtubi -f migrations/001_create_users_sessions.sql
psql -d ungdungtubi -f migrations/002_google_oauth.sql
psql -d ungdungtubi -f migrations/003_member_profile_ranks.sql
psql -d ungdungtubi -f migrations/004_storage_images_audit.sql

# 5. Chạy
cargo run
# Server: http://localhost:8080
```

### Production (qua Coolify + GitHub Actions)

Workflow deploy tự động:

1. **Push tag** `vX.Y.Z` lên GitHub → GitHub Actions trigger
2. GitHub Actions build Docker image với Rust 1.97.1
3. Push image lên `ghcr.io/mhieuhonda/ungdungtubi:vX.Y.Z` (cùng `:latest`)
4. GitHub Actions gọi Coolify webhook
5. Coolify pull image mới + redeploy lên `tubi.louis.vangioitutien.com`

**Cấu hình cần thiết trên Coolify:**
- Tạo service PostgreSQL 17 trên sub VPS 10.187.247.3
- Tạo app từ Docker image `ghcr.io/mhieuhonda/ungdungtubi:latest`
- Set env vars (xem `.env.example`)
- Bind volume `/app/static/uploads` để giữ ảnh user
- Cấu hình domain `tubi.louis.vangioitutien.com`

## Routes (v0.6)

| Method | Path | Mô tả | Auth |
|--------|------|-------|------|
| GET | `/` | Trang chủ — Không Gian | Public |
| GET | `/dang-nhap` | Trang đăng nhập (nút Google) | Public |
| POST | `/dang-nhap` | Alias chuyển hướng tới `/auth/google` | Public |
| GET | `/auth/google` | Redirect tới Google consent | Public |
| GET | `/auth/google/callback` | OAuth callback → upsert user + tạo session | Public |
| POST | `/dang-xuat` | Xoá session, redirect về `/` | Auth (POST-only để chống CSRF) |
| GET | `/ca-nhan` | Hồ sơ cá nhân + form chỉnh sửa + danh sách cấp bậc | Auth |
| POST | `/ca-nhan/cap-nhat` | Cập nhật hồ sơ | Auth |
| GET | `/cong-dong` | **[v0.6]** Trang chính Cộng Đồng (Lướt Nhóm / Lướt Chủ Đề) | Public |
| GET | `/cong-dong/tao-nhom` | **[v0.6]** Form tạo nhóm | Auth |
| POST | `/cong-dong/tao-nhom` | **[v0.6]** Tạo nhóm mới | Auth |
| GET | `/cong-dong/nhom/{slug}` | **[v0.6]** Trang nhóm + danh sách chủ đề | Public |
| POST | `/cong-dong/nhom/{slug}/tham-gia` | **[v0.6]** Tham gia nhóm | Auth |
| POST | `/cong-dong/nhom/{slug}/roi-khoi` | **[v0.6]** Rời nhóm (owner không được rời) | Auth |
| GET | `/cong-dong/nhom/{slug}/tao-chu-de` | **[v0.6]** Form tạo chủ đề | Auth + member |
| POST | `/cong-dong/nhom/{slug}/tao-chu-de` | **[v0.6]** Tạo chủ đề mới | Auth + member |
| GET | `/cong-dong/chu-de/{id}` | **[v0.6]** Trang chủ đề + bình luận | Public |
| POST | `/cong-dong/chu-de/{id}/binh-luan` | **[v0.6]** Đăng bình luận | Auth |
| GET | `/khong-gian` | Không Gian (placeholder) | Public |
| GET | `/ban-be` | Bạn Bè (placeholder) | Public |
| GET | `/kinh-sach` | Kinh Sách (placeholder) | Public |
| GET | `/quy-tu-bi` | Quỹ Từ Bi (placeholder) | Public |
| GET | `/thuong-thanh` | Thương Thành (placeholder) | Public |
| GET | `/bang-xep-hang` | Bảng Xếp Hạng (placeholder) | Public |
| GET | `/api/health` | Health check JSON + DB status | Public |
| POST | `/api/heartbeat` | Heartbeat giữ session | Auth |
| GET | `/api/upload-info` | Trả về giới hạn upload | Public |
| POST | `/api/upload-image` | Upload ảnh (5MB max, JPEG/PNG/WebP/GIF) | Auth |

## Phiên Bản

- **v0.1** — Giai đoạn 1: Nền móng hạ tầng cốt lõi
- **v0.2** — Giai đoạn 2: Hệ thống xác thực email/password
- **v0.3** — Giai đoạn 3: Chuyển sang Google OAuth (đăng nhập duy nhất bằng Google)
- **v0.4** — Giai đoạn 4: Hồ sơ thành viên & Hệ thống cấp bậc
- **v0.5** — Giai đoạn 5: Hạ tầng deploy (Docker + GitHub Actions + Coolify) + storage ảnh
- **v0.6** — Giai đoạn 6: Cộng Đồng Foundation (Nhóm + Chủ Đề + Bình luận)
- **v0.7** — Migration Actix-web → Axum 0.8 (giữ nguyên feature v0.6, Rust 1.97.1)

---

*Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.* 🪷
