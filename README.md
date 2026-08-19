# 🪷 Ứng Dụng Từ Bi

> *Siêu thoát không siêu thích. Giải thoát không giải thích. Buông bỏ mới có thể trở về.*

**Phiên bản:** v0.9.46 — Giai đoạn 61-70
**Domain:** [tubi.louis.vangioitutien.com](https://tubi.louis.vangioitutien.com)
**Stack:** Rust 1.97.1 · axum 0.8 · PostgreSQL 17 · Askama 0.14 · Alpine.js · Tailwind CSS

Ứng dụng Phật giáo Việt Nam giúp thành viên ứng dụng từ bi vào cuộc sống, tu học và giải trí — từ đó hiểu rõ hơn về khổ đau, giác ngộ và giải thoát.

---

## 📦 Mục lục

- [Tính năng](#-tính-năng)
- [Kiến trúc](#-kiến-trúc)
- [Bắt đầu nhanh](#-bắt-đầu-nhanh)
- [Cấu hình](#-cấu-hình)
- [Triển khai](#-triển-khai)
- [Roadmap](#-roadmap)
- [Lịch sử phiên bản](#-lịch-sử-phiên-bản)

---

## ✨ Tính năng

### Không Gian Cá Nhân
- **Niệm Phật Counter** — Đếm niệm Phật + theo dõi chuỗi ngày tu liên tiếp
- **Tượng Phật** — Cầu nguyện, sám hối, hồi hướng (3 loại phát nguyện)
- **Nhật Ký Tu Học** — Ghi chép hành trình tu học
- **Nhà Nhạc** — 5 thư mục (Niệm/Thiền/Đạo/Không Lời/Cá Nhân) · 4 chế độ phát · hẹn giờ tắt · tải file MP3/M4A/OGG/WAV/FLAC

### Cộng Đồng
- **Nhóm** — Tạo, tham gia, rời nhóm + logo riêng + ảnh bìa
- **Chủ Đề & Bình Luận** — Thảo luận + Live Chat WebSocket
- **Hoạt Động Cộng Đồng** — Trang `/cong-dong/hoat-dong` tổng hợp hoạt động gần đây (5 loại: topic, comment, group, music, member) với cache 5 phút

### Bạn Bè
- Kết bạn · Nhắn tin 1-1 (WebSocket) · Mail · Thông báo (đánh dấu đã đọc)

### Kinh Sách
- Thư viện Phật giáo & Đạo giáo — 5 thư viện (Phật Giáo, Đạo Giáo, Kinh Văn, Sách Quý, Quan Trọng)
- Sách + Chương + Cảm Ngộ + Tặng Hoa
- **Tìm kiếm nâng cao** — PostgreSQL Full-Text Search (tsvector + GIN) · filter chips · sort theo relevance/popular/recent · lưu lịch sử · highlight kết quả

### Thương Thành
- **Cửa hàng Ứng Dụng** — Vật phẩm hệ thống (Thẻ Tu Học, Thẻ Đổi Tên, ...)
- **Chợ Đạo Hữu** — User đăng bán vật phẩm Phật giáo · 12 danh mục + tạo danh mục mới · thanh toán K (10% phí) hoặc chuyển khoản ngân hàng
- Giỏ hàng · Checkout transaction-safe (atomic check-and-subtract)

### Tiền Tệ & Quy Đổi
- 4 loại tiền: **A** (Niệm lực) · **I** (Nguyên lực) · **K** (Tiền app) · **Bi** (Tiền Từ Bi — cao cấp)
- Trang `/tien-te` quy đổi A↔K↔Bi (transaction-safe, rate limit 10/ngày)
- Lịch sử giao dịch đầy đủ

### Quản Trị
- 5 admin roles ngang hàng (Admin Kỹ Thuật, Admin Quản Lý, Admin Cộng Đồng, Admin Phát Triển, Mod) · 150 quyền chi tiết
- Bảng kiểm duyệt: Bình luận, Nhóm, Thương Thành, Nhạc Cộng Đồng, Cảm Ngộ
- Module Từ Vựng Cấm — auto block/flag
- **Thống Kê Hệ Thống** `/admin/thong-ke` — DAU 30 ngày · signups · top tracks · top groups · categories · exchange volume · CSV export

### Hạ tầng
- Google OAuth (đăng nhập duy nhất)
- Bảng Xếp Hạng 5 tabs (A/I/K/Hôm Nay/Streak)
- Quỹ Từ Bi (đóng góp K + dashboard)
- Tìm kiếm toàn cục
- Health check `/api/health` (public minimal + admin full)

---

## 🏗 Kiến trúc

```
repo/
├── src/
│   ├── main.rs              # Router + AppState + health check
│   ├── config.rs            # Env config
│   ├── db/mod.rs            # Safety schema + helpers
│   ├── handlers/            # 17 handler modules
│   │   ├── auth.rs          # Google OAuth
│   │   ├── khong_gian.rs    # Niệm Phật + Tượng Phật + Nhật ký
│   │   ├── nha_nhac.rs      # Nhà Nhạc + music submissions
│   │   ├── community.rs     # Groups + Topics + Comments
│   │   ├── friends.rs       # Friends + DM + Mail + Notifications
│   │   ├── kinh_sach.rs     # Books + Chapters + FTS
│   │   ├── thuong_thanh.rs  # Shop + cart + transactions
│   │   ├── tien_te.rs       # Currency exchange
│   │   ├── hoat_dong.rs     # Activity feed (v0.9.44)
│   │   ├── thong_ke.rs      # Admin analytics (v0.9.44)
│   │   ├── admin.rs         # Admin dashboards + moderation
│   │   └── ...
│   ├── middleware/           # Rate limit + CSRF (no-op) + headers
│   ├── models/              # 9 model modules
│   └── static/              # CSS + JS + images
├── templates/               # Askama templates (~80 files)
├── migrations/              # 31 SQL migrations
├── Cargo.toml               # Rust 1.97.1, edition 2024
├── Dockerfile               # Multi-stage build
├── Dockerfile.coolify       # Pull pre-built image
└── docker-compose.yml       # Local dev
```

**Components chính:**
- **axum 0.8** — Web framework (HTTP + WebSocket)
- **sqlx 0.8** — PostgreSQL driver + compile-time checked queries + migrations
- **askama 0.14** — Type-safe HTML templates
- **tokio** — Async runtime
- **tower-http** — Compression + CORS + static files + tracing
- **Alpine.js 3.14** — Frontend reactivity (via CDN)
- **Tailwind CSS** — Styling (via CDN)

---

## 🚀 Bắt đầu nhanh

### Yêu cầu
- Rust 1.97.1 (đã pin trong `Cargo.toml` + `Dockerfile`)
- PostgreSQL 17+ (cho dev local)
- Google OAuth credentials (cho login)

### Chạy local

```bash
# 1. Clone
git clone https://github.com/mhieuhonda/ungdungtubi.git
cd ungdungtubi

# 2. Cấu hình env (xem .env.example)
cp .env.example .env
# Edit .env: DATABASE_URL, GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, GOOGLE_REDIRECT_URI, ...

# 3. Chạy PostgreSQL (docker-compose hoặc external)
docker-compose up -d postgres

# 4. Chạy migrations + safety schema (tự động khi khởi động app với APP_ENV=production hoặc RUN_MIGRATIONS=true)
APP_ENV=production cargo run
```

Server chạy tại `http://127.0.0.1:8080`.

### Health check

```bash
curl http://127.0.0.1:8080/api/health
# {"status":"ok","version":"0.9.44","app":"Ứng Dụng Từ Bi"}
```

---

## ⚙️ Cấu hình

| Env var | Mô tả | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | — (bắt buộc) |
| `GOOGLE_CLIENT_ID` | Google OAuth client ID | — (bắt buộc) |
| `GOOGLE_CLIENT_SECRET` | Google OAuth client secret | — (bắt buộc) |
| `GOOGLE_REDIRECT_URI` | OAuth callback URL | `http://localhost:8080/auth/google/callback` |
| `APP_BASE_URL` | Public app URL (cho links) | `http://localhost:8080` |
| `DOMAIN` | Public domain (cho SEO/CSP) | `localhost` |
| `HOST` / `PORT` | Bind address | `127.0.0.1` / `8080` |
| `UPLOAD_DIR` | Upload directory | `./uploads` |
| `UPLOAD_URL_PREFIX` | URL prefix cho uploaded files | `/static/uploads` |
| `MAX_UPLOAD_BYTES` | Max upload size (bytes) | `5242880` (5MB) |
| `DB_MAX_CONNECTIONS` | PgPool max connections | `10` |
| `APP_ENV` | `production` để tự chạy migrations | `development` |
| `RUN_MIGRATIONS` | `true` để force run migrations | — |
| `TZ` | Timezone (cho chrono::Local) | `Asia/Ho_Chi_Minh` (set trong Dockerfile) |

Xem `.env.example` để biết đầy đủ.

---

## 🚢 Triển khai

### Coolify (production)

App deploy qua [Coolify](https://coolify.b1449.com) lên sub VPS:

- **Image:** `ghcr.io/mhieuhonda/tubi-app:latest` (build bởi GitHub Actions, pull bởi Coolify)
- **Database:** PostgreSQL 17 chạy riêng trên sub VPS
- **Domain:** `tubi.louis.vangioitutien.com` (Traefik reverse proxy + Let's Encrypt)
- **Healthcheck:** `GET /api/health` (port 8080, scheme http, retries 5, start-period 60s)
- **Volume:** `/app/static/uploads` để giữ ảnh user upload
- **Auto-migrations:** Chạy khi khởi động (`RUN_MIGRATIONS=true`, `APP_ENV=production`)
- **Timezone:** `TZ=Asia/Ho_Chi_Minh` (set trong Dockerfile)

### CI/CD

- **GitHub Actions** build → push image lên GHCR → trigger Coolify deploy (webhook POST)
- Workflow tự update `Dockerfile.coolify` SHA tag trước khi trigger Coolify → tránh Docker cache stale digest

### GitHub Secrets cần thiết

- `COOLIFY_API_TOKEN` — API token của Coolify (User Settings → API Tokens)
- `COOLIFY_APP_UUID` — UUID của application trên Coolify

### Rollback

Đổi tag image trong Coolify từ `:latest` sang `:sha-<old>` hoặc `:v0.9.43` → deploy lại.

---

## 🗺 Roadmap

Theo tài liệu `HieuLouis/Kế Hoạch Và Mục Tiêu Phát Triển Dự Án Từ Bi.docx`:

### Giai đoạn I — Kiến tạo nền móng (6 tháng)
- ✅ Tuyên ngôn + triết lý cốt lõi
- ✅ Hạ tầng deploy (Docker + GitHub Actions + Coolify)
- ✅ Module cốt lõi: Không Gian, Cộng Đồng, Bạn Bè, Kinh Sách, Thương Thành, Nhà Nhạc
- ✅ Hệ thống admin + 150 quyền chi tiết
- 🔄 Close beta → Open beta (đang chuẩn bị)

### Giai đoạn II — Phát triển hệ sinh thái (100 ngày)
- 🎯 10.000 người dùng · 50.000 cộng đồng · 100 tình nguyện viên
- 🎯 Phiên bản 1.1 → 1.2 → hoàn mỹ
- 🎯 Truyền thông đa kên (Facebook, YouTube, TikTok, Discord)

### Giai đoạn III — Toàn cầu hóa (1000 ngày)
- 🌍 Quốc tế hóa (EN, ZH, JA, KO)
- 🌍 Cộng đồng quốc tế + đại sứ cộng đồng
- 🌍 Hệ sinh thái hoàn thiện: Ứng Dụng Từ Bi + Cộng Đồng Từ Bi + Game Siêu Độ + AI Từ Bi + Học viện Từ Bi

Xem roadmap trực quan tại `/admin/phat-trien` (yêu cầu admin role).

---

## 📜 Lịch sử phiên bản

| Phiên bản | Giai đoạn | Tóm tắt |
|---|---|---|
| **v0.9.44** | **48-52** | Music Approval Hardening + Notification Polish + Hoạt Động Cộng Đồng + Kinh Sách FTS + Admin Analytics |
| v0.9.43 | 47 | Currency Exchange (A↔K↔Bi) + Music Submit DB Hardening + Coolify Webhook POST |
| v0.9.42 | 46 | Forbidden Words Auto-Check + Hệ Thống Tiền Tệ Bi + Balance UI |
| v0.9.41 | 45 | Admin Moderation Hoàn Thiện + Từ Vựng Cấm + Heartbeat Fix |
| v0.9.40 | 44 | Chợ Đạo Hữu + Admin Thương Thành Hoàn Thiện + Payment K/Bank |
| v0.9.39 | 43 | Active User Sync + Settings Fix + Stats Timezone Fix + Mobile Menu Accordion |
| v0.9.38 | 42 | Logo PNG + Group Logo Bug Fix + Music Submit Bug Fix + About Page Team Update |
| v0.9.37 | 41 (phần 2) | About Page + Orphan-Link Fix + Post-Submit Fix + Notification Mark-All + 429 Hardening |
| v0.9.36 | 41 | Community Group Logo + Audio File Uploads |
| v0.9.35 | 40 | Nhạc Cộng Đồng (YouTube) + Game Cleanup |
| v0.9.34 | 39 | Thương Thành MVP + Cart + K transactions |
| v0.9.33 | 38 | Nhà Nhạc (5 thư mục + 5 chế độ phát) |
| v0.9.32 | 37 | Admin Phát Triển Dashboard |
| v0.9.31 | 36 | About page + Admin music pending UI polish |
| v0.9.30 | 35 | Money System Hardening |
| v0.9.29 | 34 | Quản Lý Thương Thành |
| v0.9.28 | 33 | Mobile UI Overhaul |
| v0.9.27 | 32 | Live Chat Cộng Đồng Fix |
| v0.9.26 | 31 | Deploy Pipeline Fix |
| v0.9.25 | 30 | Friends Notification Polish |
| v0.9.24 | 29 | Security Middleware (rate limit + CSRF + headers) |
| v0.9.23 | 28 | Settings Polish + Heartbeat |
| v0.9.22 | 27 | Profile Polish |
| v0.9.21 | 26 | Mod Role |
| v0.9.20 | 25 | Live Chat Total Fix + Sound Effects |
| v0.9.19 | 24 | Live Chat Cộng Đồng Fix + Mod role |
| v0.9.18 | 23 | Mobile UI Overhaul |
| v0.9.16 | 21 | UI Redesign + Route Hub |
| v0.9.14 | 18-19 | Navigation Overhaul + 150 Permissions + Achievements + Search |
| v0.9.11 | 15 | Quỹ Từ Bi |
| v0.9.10 | 14 | Bảng Xếp Hạng |
| v0.9.9 | 13 | Không Gian Cá Nhân |
| v0.9.8 | 12 | 50 quyền chi tiết + 3 admin dashboards |
| v0.9.7 | 11 | Hệ thống vai trò Admin |
| v0.9.6 | 10 | Kinh Sách |
| v0.9.5 | 9 | Module Bạn Bè |
| v0.9.4 | 8 | CI/CD GitHub Actions |
| v0.9.3 | 7 | Live Chat WebSocket |
| v0.9.2 | 7 | Chat Chung toàn platform |
| v0.9.1 | 6 | UI mobile + Coolify manual deploy |
| v0.9 | 9 | Codebase sạch lỗi (clippy pedantic/nursery pass, axum 0.8 stable) |
| v0.6 | 6 | Cộng Đồng Foundation |
| v0.5 | 5 | Hạ tầng deploy |
| v0.4 | 4 | Hồ sơ + Hệ thống cấp bậc |
| v0.3 | 3 | Google OAuth |
| v0.2 | 2 | Email/password auth |
| v0.1 | 1 | Nền móng hạ tầng cốt lõi |

Chi tiết đầy đủ: [`CHANGELOG.md`](CHANGELOG.md)

---

## 🤝 Đóng góp

Dự án tuân theo triết lý **"Phát triển cộng đồng trước, phát triển công nghệ sau"**. Vui lòng đọc tài liệu trong `HieuLouis/` để hiểu triết lý và định hướng dự án.

Mọi đóng góp đều được hoan nghênh — từ báo lỗi, dịch thuật, đến feature PR.

---

*Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.* 🪷
