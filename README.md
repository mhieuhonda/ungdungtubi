# 🪷 Ứng Dụng Từ Bi

> *Siêu thoát không siêu thích. Giải thoát không giải thích. Buông bỏ mới có thể trở về.*

**Domain:** [tubi.louis.vangioitutien.com](https://tubi.louis.vangioitutien.com)

## Tầm Nhìn

Xây dựng một hệ sinh thái giúp mọi người có thể ứng dụng Từ Bi vào cuộc sống, tu học và giải trí, từ đó hiểu rõ hơn về bản chất của khổ đau, giác ngộ và giải thoát.

**Triết lý cốt lõi:** Tu cũng niệm Phật. Chơi cũng niệm Phật.

## Công Nghệ

| Thành phần | Công nghệ |
|-----------|-----------|
| Backend | Rust 1.97.1 + Actix-web |
| Template | Askama 0.14 (type-safe HTML templates) |
| Database | PostgreSQL + SQLx (async, compile-time checked) |
| Frontend | HTMX (server-driven UI) + Alpine.js (reactive) |
| Styling | Tailwind CSS |
| Auth | Google OAuth 2.0 (OpenID Connect — userinfo) — đăng nhập duy nhất |

## 4 Chuyên Mục Chính

1. 🌍 **Không Gian** – Không gian cá nhân, cộng tu, niệm Phật
2. 👥 **Cộng Đồng** – Diễn đàn, nhóm, chủ đề, live chat
3. 👤 **Bạn Bè** – Kết nối, nhắn tin, gửi thư
4. 📚 **Kinh Sách** – Thư viện kinh sách Phật giáo & Đạo giáo

---

## Lộ Trình 25 Giai Đoạn Phát Triển

### Giai đoạn 1: Kiến tạo nền móng — Thiết lập dự án & hạ tầng cốt lõi ✅ (v0.1)
- Khởi tạo project Rust (Actix-web + Askama + SQLx + PostgreSQL)
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

### Giai đoạn 5–25: *(xem kế hoạch chi tiết trong HieuLouis/)*

---

## Cấu Trúc Dự Án (Giai đoạn 4 / v0.4)

```
ungdungtubi/
├── src/
│   ├── main.rs              # Entry point + routes Google OAuth + hồ sơ + session cleanup
│   ├── config.rs            # Config (DB, host, Google OAuth, production safety)
│   ├── db/
│   │   └── mod.rs           # Database helpers (session cleanup)
│   ├── errors/
│   │   └── mod.rs           # AppError enum with HTTP response mapping
│   ├── handlers/
│   │   ├── mod.rs           # Page handlers + session auth helper + profile update
│   │   └── auth.rs          # google_login, google_callback, logout
│   ├── models/
│   │   ├── mod.rs
│   │   └── user.rs          # User, GoogleUserInfo, MemberRank, ProfileUpdate
│   └── static/
│       ├── css/app.css
│       └── js/app.js
├── templates/                # Askama templates (Vietnamese)
│   ├── layout.html
│   ├── home.html
│   ├── profile.html         # Trang hồ sơ + form chỉnh sửa + danh sách cấp bậc (Giai đoạn 4)
│   └── auth/
│       └── login.html        # Chỉ còn nút "Đăng nhập bằng Google"
├── migrations/
│   ├── 001_create_users_sessions.sql
│   ├── 002_google_oauth.sql  # password_hash NULL, google_sub, avatar_url, email_verified
│   └── 003_member_profile_ranks.sql  # phap_danh/phap_hieu/but_danh/gender/bio + bảng member_ranks
├── HieuLouis/                # Tài liệu dự án
├── Cargo.toml
├── .env.example              # Template cấu hình môi trường
└── README.md
```

## Cài Đặt & Chạy

```bash
# 1. Clone
git clone https://github.com/mhieuhonda/ungdungtubi.git
cd ungdungtubi

# 2. Tạo .env từ .env.example, điền DATABASE_URL + GOOGLE_CLIENT_ID + GOOGLE_CLIENT_SECRET
cp .env.example .env
# Cập nhật GOOGLE_REDIRECT_URI cho khớp với Google Console
#   Local:    http://localhost:8080/auth/google/callback
#   Prod:     https://tubi.louis.vangioitutien.com/auth/google/callback

# 3. Cấu hình Google OAuth
#    - Vào Google Cloud Console → APIs & Services → Credentials
#    - Tạo OAuth 2.0 Client ID (Web application)
#    - Thêm Authorized redirect URIs khớp với GOOGLE_REDIRECT_URI
#    - Copy Client ID + Client Secret vào .env

# 4. Tạo database + chạy migrations
createdb ungdungtubi
psql -d ungdungtubi -f migrations/001_create_users_sessions.sql
psql -d ungdungtubi -f migrations/002_google_oauth.sql
psql -d ungdungtubi -f migrations/003_member_profile_ranks.sql

# 5. Chạy
cargo run
# Server: http://localhost:8080
```

## Routes (v0.4)

| Method | Path | Mô tả | Auth |
|--------|------|-------|------|
| GET | `/` | Trang chủ — Không Gian | Public |
| GET | `/dang-nhap` | Trang đăng nhập (nút Google) | Public |
| POST | `/dang-nhap` | Alias chuyển hướng tới `/auth/google` | Public |
| GET | `/auth/google` | Redirect tới Google consent | Public |
| GET | `/auth/google/callback` | OAuth callback → upsert user + tạo session | Public |
| POST | `/dang-xuat` | Xoá session, redirect về `/` | Auth |
| GET | `/dang-xuat` | Alias của POST (cho link đơn giản) | Auth |
| GET | `/ca-nhan` | Hồ sơ cá nhân + form chỉnh sửa + danh sách cấp bậc | Auth (redirect → /dang-nhap nếu chưa) |
| POST | `/ca-nhan/cap-nhat` | Cập nhật hồ sơ | Auth |
| GET | `/khong-gian` | Không Gian (placeholder) | Public |
| GET | `/cong-dong` | Cộng Đồng (placeholder) | Public |
| GET | `/ban-be` | Bạn Bè (placeholder) | Public |
| GET | `/kinh-sach` | Kinh Sách (placeholder) | Public |
| GET | `/quy-tu-bi` | Quỹ Từ Bi (placeholder) | Public |
| GET | `/thuong-thanh` | Thương Thành (placeholder) | Public |
| GET | `/bang-xep-hang` | Bảng Xếp Hạng (placeholder) | Public |
| GET | `/api/health` | Health check JSON | Public |
| POST | `/api/heartbeat` | Heartbeat giữ session | Auth |

## Phiên Bản

- **v0.1** — Giai đoạn 1: Nền móng hạ tầng cốt lõi
- **v0.2** — Giai đoạn 2: Hệ thống xác thực email/password
- **v0.3** — Giai đoạn 3: Chuyển sang Google OAuth (đăng nhập duy nhất bằng Google)
- **v0.4** — Giai đoạn 4: Hồ sơ thành viên & Hệ thống cấp bậc

---

*Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.* 🪷
