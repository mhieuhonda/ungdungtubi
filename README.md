# 🪷 Ứng Dụng Từ Bi

> *Siêu thoát không siêu thích. Giải thoát không giải thích. Buông bỏ mới có thể trở về.*

**Domain:** [tubi.louis.vangioitutien.com](https://tubi.louis.vangioitutien.com)

## 📦 Phiên bản hiện tại: v0.9.15 — Giai đoạn 20

**Giai đoạn 20: Niệm Phật Fix + Admin Redesign + Mobile UX**

### Bug fixes critical
- **Niệm Phật counter không bị lệch trái** sau click (HTMX response giữ nguyên class `text-center mb-4`)
- **Streak (số ngày tu liên tiếp) tính đúng** — fix timezone mismatch giữa `chrono::Local::now()` và `CURRENT_DATE` của PostgreSQL trong Docker (TZ=UTC)
- **Tổng niệm / niệm hôm nay cập nhật ngay lập tức** sau khi niệm — dùng HTMX `hx-swap-oob` để swap nhiều element cùng lúc (counter + 4 stats card + footer)
- **Form Cầu Nguyện / Sám Hối / Hồi Hướng gửi được** — fix bug `hx-post=""` (rỗng) chặn submit. Tách thành 3 form riêng biệt với `hx-post` URL cố định
- **`practice_logs` upsert không còn nuốt error** — log + rollback nếu fail (trước đây `let _ =` làmtoday_niem không cập nhật dù a_balance vẫn tăng)

### UI/UX redesign
- **Bảng quản trị Admin Kỹ Thuật redesign theo ảnh tham chiếu** — dark theme, mobile-first, 2-col stats grid (số đỏ coral) + 2-col nav tiles grid
- **Hiển thị đúng 150 quyền** — admin_ky_thuat có 150 quyền (trước đây hiển thị hardcoded "6/50")
- **15 nav tiles điều hành** — Hướng dẫn, Phê duyệt, Thành viên, Nhóm, Kinh sách, Báo cáo, Bình luận, Từ vựng cấm, Nội dung đánh dấu, Quản lý tag, VIP, Quỹ Từ Bi, Bảng xếp hạng, Nhật ký, Health check
- **Permission matrix 10 nhóm × 10 quyền** — hiển thị đầy đủ 150 quyền chia 10 nhóm (system, users, content, community, kinh_sach, fund, achievements, security, media, analytics)

### Navigation overhaul (theo yêu cầu user)
- **Menu 3 gạch rút gọn chỉ còn 7 mục**: Không Gian, Cộng Đồng, Bạn Bè, Kinh Sách, Hồ Sơ, Quản Trị (nếu admin), Thoát
- **Bottom nav đổi icon**: Trang Chủ → 🏠 ngôi nhà, nút giữa → 🪷 hoa sen (Tổng Quan)
- **Các mục khác phân bổ hợp lý**: Tổng Quan/Quỹ/Bảng Xếp Hạng/Thành Tích/Thương Thành/Tìm Kiếm → truy cập qua nút giữa 🪷; Cài Đặt/Tin Nhắn/Hộp Thư/Thông Báo/Tìm Bạn → trong trang /ban-be và /ca-nhan; Kinh Phật/Kinh Đạo/Tìm Sách/Tạo Nhóm → trong trang /kinh-sach và /cong-dong

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
| Container | Docker (multi-stage build với Rust 1.97.1-slim-bookworm, image final ~30 MB) |
| CI/CD | GitHub Actions (build → push GHCR) + Coolify API (auto pull image → deploy) |
| Registry | GHCR (ghcr.io/mhieuhonda/tubi-app) — image public |

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
- **Coolify** deploy trên sub VPS, domain `tubi.louis.vangioitutien.com`
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
- **Mục tiêu:** Web chạy production ổn định, sẵn sàng cho giai đoạn 6+

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

### Giai đoạn 7: Live Chat WebSocket trong Nhóm ✅ (v0.9.2)
- **Live Chat real-time (WebSocket) trong Nhóm** — điểm khác biệt cốt lõi của Cộng Đồng Ứng Dụng Từ Bi so với Telegram/Zalo/Facebook Group
- Theo thiết kế `HieuLouis/Giao Diện Cộng Đồng Trong Ứng Dụng.docx`: mỗi nhóm có Danh sách Chủ Đề (diễn đàn, ~65% chiều cao) + Live Chat (real-time, ~35% chiều cao)
- **Triết lý**: Live Chat chỉ để giao lưu / kết bạn / tán gẫu / hỏi nhanh. Mọi nội dung có giá trị nên được chuyển thành Chủ Đề (lưu trữ tri thức lâu dài)
- **WebSocket endpoint** `GET /ws/cong-dong/nhom/{slug}` (Axum 0.8 `WebSocketUpgrade`)
  - Auth bằng session_id cookie, chỉ member active mới chat được
  - ChatHub quản lý `broadcast::Sender` per-group, capacity 256
  - Spawn 2 task song song: send (broadcast→client) + recv (client→DB→broadcast)
- **REST endpoint** `GET /api/cong-dong/nhom/{slug}/chat-history?limit=50&before={iso8601}` — paginated history
- **Migration 006**: bảng `group_chat_messages` (body VARCHAR(500), 2 index)
- **Frontend**: Alpine.js `liveChat()` component — auto-reconnect exponential backoff, auto-scroll, formatTime vi-VN
- **Mục tiêu:** Thành viên trong nhóm có thể chat real-time, kết nối cộng đồng

### Giai đoạn 8: CI/CD tự động — GitHub Actions + Docker Image + Coolify ✅ (v0.9.4)
- **Quay lại GitHub Actions** nhưng với mô hình mới: GitHub Actions build & push Docker Image lên GHCR, Coolify auto pull image và deploy (không còn build từ source trên VPS như v0.9.1)
- **Workflow `.github/workflows/docker.yml`**:
  - Trigger: push lên `main` hoặc tag `v*`
  - Build multi-stage Docker image với Rust 1.97.1-slim-bookworm (image final ~30 MB)
  - Push lên GHCR (`ghcr.io/mhieuhonda/tubi-app`) với multi-tag: `latest`, `sha-<short>`, `vX.Y.Z`, `vX.Y`
  - Buildx cache (type=gha) để tăng tốc build sau
  - Gọi Coolify API `/api/v1/applications/{uuid}/start` để trigger deploy
  - Coolify nhận yêu cầu → pull image `:latest` → redeploy container
- **Coolify app** chuyển từ `build_pack: dockerfile` (build từ source) sang `build_pack: dockerimage` (pull image từ registry)
- **GitHub Secrets**: `COOLIFY_API_TOKEN`, `COOLIFY_APP_UUID`
- **Lợi ích so với v0.9.1**:
  - Build trên GitHub-hosted runner (không tốn CPU/RAM VPS)
  - Image đã build sẵn, deploy chỉ mất vài giây (pull + restart)
  - Rollback dễ dàng: đổi tag trong Coolify về `sha-<old>` hoặc `v0.9.3`
  - Multi-arch support sẵn sàng (chỉ cần thêm `platforms: linux/amd64,linux/arm64`)
  - Image có thể ký (provenance/SBOM optional)
- **Mục tiêu:** Push code → tự động build & deploy trong < 5 phút, không cần thao tác thủ công

### Giai đoạn 9: Module Bạn Bè — Friends + DM + Mail + Notifications ✅ (v0.9.5)
- **BB-01 Kết bạn (Friend System)**: gửi/nhận/từ chối/hủy lời mời kết bạn; tìm user theo tên/email/pháp danh
- **BB-02 Nhắn tin 1-1 (Direct Messaging)**: WebSocket realtime `/ws/ban-be/tin-nhan/{conversation_id}` (reuse ChatHub pattern, `DmChatHub` per-conversation capacity 128), REST history paginated, `dmChat()` Alpine.js component với auto-reconnect backoff
- **BB-03 Gửi thư (Mail/Inbox)**: thư dài (subject max 200 + body TEXT), hộp thư đến/đi, auto mark as read, notifications cho recipient
- **Notification Center**: bảng `notifications` (JSONB payload), bell icon ở header với red badge (poll mỗi 30s), types: friend_request/friend_accept/mail/dm/system/group_invite
- **4 migration mới**: `008_friendships.sql`, `009_conversations_direct_messages.sql`, `010_mails.sql`, `011_notifications.sql`
- **Bug fixes**: Fix live chat tổng luôn báo "đang kết nối..." (bỏ check `document.cookie.includes('session_id')` vì cookie HttpOnly không đọc được qua `document.cookie`); thêm server-side logging cho WS errors; fix version strings v0.9.3 → v0.9.5
- **Mục tiêu:** Thành viên có thể kết bạn, nhắn tin realtime 1-1, gửi thư dài, nhận thông báo — bù lỗ hổng social của v0.9.4

### Giai đoạn 13: Không Gian Cá Nhân & Niệm Phật ✅ (v0.9.9)
- **Chuyên mục Không Gian chính thức ra mắt** — không còn placeholder, là 1 trong 4 trụ cột chính của app
- **Niệm Phật Counter** (HTMX realtime):
  - Nút 🪷 lớn ở giữa trang, mỗi lần nhấn = +1 Niệm Lực A
  - `POST /api/niem-phat` — upsert `practice_logs` (1 row/user/day) + increment `a_balance` trong transaction
  - Hiệu ứng pulse lotus + scale animation khi nhấn
- **Tượng Phật** (4 chức năng theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục I.6):
  - 🙏 **Cầu Nguyện** — `POST /tuong-phat/cau-nguyen` → +1 Nguyên lực I
  - 🙇 **Sám Hối** — `POST /tuong-phat/sam-hoi` → +2 Nguyên lực I
  - 🌸 **Hồi Hướng** — `POST /tuong-phat/hoi-huong` → +3 Nguyên lực I
  - (Ủng Hộ chưa làm — cần tích hợp quảng cáo, sẽ thêm ở giai đoạn sau)
  - Form vow: nội dung 10–2000 ký tự, có checkbox "hiển thị công khai trên bảng Kính Nguyện"
- **Nhật Ký Tu Học** (7 ngày gần nhất):
  - Biểu đồ cột (bar chart) hiển thị số lần niệm mỗi ngày
  - Tính `streak` (số ngày liên tiếp có niệm)
- **Bảng Kính Nguyện** — danh sách vow công khai gần nhất (20 cái), hiển thị tác giả + loại vow + nội dung
- **Hệ thống điểm mở rộng**:
  - `a_balance` (Niệm Lực A) — có từ Giai đoạn 1
  - `k_balance` (Tiền K) — có từ Giai đoạn 1
  - **`i_balance` (Nguyên lực I) — MỚI v0.9.9** — phần thưởng từ Tượng Phật
- **Migration 015**: thêm cột `i_balance` vào `users` + bảng `practice_logs` + bảng `buddha_vows` + triggers + index
- **5 endpoint** mới cho Không Gian + 1 API JSON stats
- **Bug fix CRITICAL (login "lỗi ghi nhận người dùng")**:
  - `auth.rs` `upsert_google_user` thêm `SELECT` fallback sau khi `INSERT ... RETURNING` fail (tránh block login khi struct/column mismatch)
  - Truncate `display_name` về 100 ký tự (Google profile name có thể dài hơn `VARCHAR(100)`)
  - Log chi tiết lỗi theo loại (`ColumnNotFound` / `Database` / `Decode`) để debug triệt để
- **Admin user list redesign (theo ảnh tham chiếu)**:
  - Bỏ table layout, chuyển sang card-based compact grid (2 cột trên desktop)
  - Mỗi card: tên (bold) + role badge (top-right), @handle, email, footer với A/K metrics + online status ("Đang hoạt động" / "X phút trước" / "Bị khóa")
  - Actions dropdown (⋮) thay vì select inline — UI gọn hơn
  - `last_session_at` từ subquery `MAX(sessions.created_at)` để hiển thị hoạt động gần nhất
- **Mục tiêu:** Không Gian cá nhân hoạt động — niệm Phật tích lũy A, phát nguyện trước Tượng Phật tích lũy I, theo dõi nhật ký tu học hàng ngày

### Giai đoạn 12: 50 quyền chi tiết + 3 giao diện admin riêng biệt ✅ (v0.9.8)
- **Hệ thống 50 quyền chi tiết (Granular Permissions)** — 5 nhóm x 10 quyền:
  - `system` (10) — hệ thống, server, DB, config, logs, metrics, backup, debug
  - `users` (10) — xem, sửa, đổi role, ban, xóa, sessions, OAuth, export
  - `content` (10) — duyệt, sửa, xóa, ghim, khoá, danh mục, tags, mod comments/reviews
  - `community` (10) — nhóm, events, chat, members, broadcast, invites, archive, merge
  - `kinh_sach` (10) — sách, chương, upload, danh mục, reviews, donations, mail, notif, analytics, API
- **Nâng Admin Kỹ Thuật lên CHỨC VỤ CAO NHẤT** — toàn bộ 50 quyền
  - Hierarchy mới: Admin Kỹ Thuật (50 quyền) > Admin Quản Lý (30 quyền) > Admin Cộng Đồng (20 quyền) > Thành Viên (0)
- **3 giao diện bảng quản trị hoàn toàn khác nhau**:
  - ⚙️ `/admin/ky-thuat` — Phong cách Coder/Terminal — tối, Matrix-like, cực ngầu
  - 🛡️ `/admin/cong-dong` — Phong cách Community Mod — xanh dương, social, ấm áp
  - 👑 `/admin/quan-li` — Phong cách Executive/Premium — vàng, luxury dashboard
- **Migration 014**: bảng `permissions` + `role_permissions` + view `v_user_permissions` + function `user_has_permission()`
- **Bug fix CRITICAL**: `USER_COLUMNS` trong auth.rs thiếu cột `role` — Google OAuth login bị hỏng
- **Mục tiêu:** Admin Kỹ Thuật có toàn quyền, 3 kiểu admin có giao diện riêng, hệ thống phân quyền chi tiết

### Giai đoạn 11: Hệ thống vai trò Admin & Phân quyền cộng đồng ✅ (v0.9.7)
- **Lần đầu tiên app có hệ thống phân quyền rõ ràng** — 4 vai trò:
  - `member` — Thành Viên (mặc định)
  - `admin_ky_thuat` — Admin Kỹ Thuật (hệ thống, server, DB, mã nguồn)
  - `admin_cong_dong` — Admin Cộng Đồng (duyệt nội dung, mod diễn đàn)
  - `admin_quan_li` — Admin Quản Lý (super admin — quyền cao nhất)
- **Hierarchy:** Admin Quản Lý > Admin Cộng Đồng > Admin Kỹ Thuật > Thành Viên
- **Migration 013**: thêm cột `role` vào `users` + CHECK constraint + index
  + UPSERT `khongdich.admin@gmail.com` → `admin_ky_thuat`
- **Hiển thị chức vụ trên hồ sơ** (`/ca-nhan`) — role badge bên cạnh rank badge,
  trong bảng Thông Tin Tài Khoản, và nút "Vào trang Quản Trị" (chỉ admin)
- **Hiển thị chức vụ trong header** — role badge nhỏ + link "⚙️ Quản Trị"
  (chỉ admin nhìn thấy)
- **Trang Quản Trị** `/admin`:
  - Dashboard: tổng users, active users, admin count, groups/topics/comments/books/mails,
    cảm ngộ chờ duyệt
  - `/admin/thanh-vien` — Danh sách thành viên + UI đổi role (chỉ Admin Quản Lý)
  - `POST /admin/thanh-vien/{user_id}/role` — Đổi role user
  - Permission gate: user không phải admin → 403 Forbidden
- **User model** mở rộng: 8 helper methods (`role_display`, `role_icon`, `role_color`,
  `role_level`, `is_admin`, `is_admin_ky_thuat`, `is_admin_cong_dong`, `is_admin_quan_li`)
- **Health check** `/api/health` thêm `admin` stats + `roles` object
- **Mục tiêu:** Hệ thống có admin rõ ràng, có trang quản trị cơ bản, nền tảng
  cho các giai đoạn tiếp theo (duyệt cảm ngộ, mod community, ban user, ...)

### Giai đoạn 10: Kinh Sách — Thư viện kinh sách Phật giáo & Đạo giáo ✅ (v0.9.6)
- **Chuyên mục Kinh Sách chính thức ra mắt** — không còn placeholder
- **5 Thư Viện Chính** (theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục IV):
  - 🪷 Phật Gia — Kinh điển, luận thư, pháp thoại Phật giáo
  - ☯️ Đạo Gia — Đạo Đức Kinh, Nam Hoa Kinh, thư tịch Đạo giáo
  - 📜 Kinh Văn — Kinh văn tụng đọc, chú giải, nghi thức
  - 💎 Sách Quý — Khoa học, Triết học, Tâm học, Văn học
  - ⭐ Quan Trọng — Bài viết quan trọng do Quản Lý chọn lựa
- **Hệ thống Sách (Books)**:
  - Sách điện tử hoàn chỉnh (`book_type=single`) hoặc theo chương (`book_type=multi`)
  - Trang sách `/kinh-sach/{slug}` hiển thị thông tin + danh sách chương + cảm ngộ
  - Tự tăng `view_count` khi xem sách/chương
  - Phân loại theo 5 thư viện + 3 ngôn ngữ (vi/en/zh, ưu tiên Tiếng Việt)
- **Hệ thống Chương (Chapters)**:
  - Trang đọc chương `/kinh-sach/{slug}/chuong/{chapter_slug}` với sidebar mục lục sticky
  - Điều hướng trước/sau giữa các chương
  - Tự tăng `view_count` chương khi đọc
- **Hệ thống Cảm Ngộ (Reviews)**:
  - Form cảm ngộ ngay trên trang sách (tối thiểu 100 chữ, tối đa 10.000 chữ)
  - **Phải qua xét duyệt** mới hiển thị công khai (status: `pending` → `approved`)
  - 1 user chỉ được viết 1 cảm ngộ/sách (unique index), có thể edit
  - Hiển thị trạng thái cảm ngộ của user (chờ duyệt / đã duyệt / bị từ chối)
- **Tặng Hoa (Flowers)**:
  - 1 user chỉ tặng 1 hoa/sách (unique index `book_flowers`)
  - Tự tăng counter `flower_count` qua trigger
- **Tìm kiếm sách** `/kinh-sach/tim-kiem?q=` — dùng ILIKE + pg_trgm (fuzzy search)
- **Migration 012**: 5 bảng mới (`book_categories`, `books`, `book_chapters`, `book_reviews`, `book_donations`, `book_flowers`) + 5 triggers + seed 4 cuốn sách mẫu (Kinh A Di Đà, Đạo Đức Kinh, Kinh Tam Đại Hải, Kinh Pháp Cú)
- **12 endpoint** mới cho Kinh Sách
- **Health check** bổ sung `kinh_sach` stats (số sách, chương, cảm ngộ, tổng view)
- **Mục tiêu:** Thành viên có thể đọc kinh sách online, viết cảm ngộ, tặng hoa kính dâng — bù lỗ hổng kiến thức của v0.9.5

### Giai đoạn 15: Quỹ Từ Bi — Hệ thống quỹ cộng đồng ✅ (v0.9.11)
- **Chuyên mục Quỹ Từ Bi chính thức ra mắt** (`/quy-tu-bi`) — theo `HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx` mục VI
- **Hệ thống đóng góp K vào quỹ**:
  - `POST /quy-tu-bi/dong-gop` — form đóng góp K (trừ từ `k_balance` của user, transaction-safe)
  - 5 loại quỹ: 🪷 Quỹ Chung · 📚 Quỹ Sách · 🕉️ Quỹ Tu · 🎁 Quỹ Quà · 🤝 Quỹ Thiện Nguyện
  - Hỗ trợ đóng góp ẩn danh, lời nhắn tùy chọn (max 500 ký tự)
  - Notification tự động cho admins khi có donation mới
- **Dashboard tổng quan**:
  - Hero số dư Quỹ (gradient xanh tubi, chữ vàng amber)
  - Stats grid: tổng K/A/I hệ thống · tổng lượt đóng góp
  - Quỹ theo chuyên mục: 5 card màu
  - Top 10 nhà hảo tâm (medal 🥇🥈🥉)
  - 20 đóng góp gần nhất (table với avatar + loại quỹ badge + lời nhắn)
  - 10 khoản chi tiêu gần đây (công khai, minh bạch)
- **Migration 016**: 3 bảng mới (`fund_donations`, `fund_campaigns`, `fund_expenses`) + view `v_fund_summary` + 6 index + 2 trigger
- **API endpoint** `GET /api/quy-tu-bi/stats` — JSON tổng quan
- **Bug fix CRITICAL (lỗi đăng nhập)**:
  - Fix production vẫn chạy v0.9.9 dù v0.9.10 đã build & push lên GHCR
  - Nguyên nhân: `Dockerfile.coolify` dùng `FROM :latest` → Docker daemon cache stale digest
  - Giải pháp: đổi sang `FROM :0.9.11` (tag semver unique → Docker chắc chắn pull image mới)
  - Đảm bảo v0.9.10's safety schema fix được deploy (fix lỗi "column i_balance does not exist")
- **Mục tiêu:** Thành viên có thể đóng góp K vào quỹ chung, công khai minh bạch, fix triệt để lỗi đăng nhập

### Giai đoạn 12–25: *(xem kế hoạch chi tiết trong HieuLouis/)*

### Giai đoạn 18: Navigation Overhaul + User Hub + Settings ✅ (v0.9.14)
- **Trang Tổng Quan (User Hub) `/tong-quan`** — liệt kê TẤT CẢ 24 tính năng của app trong 4 nhóm (Chuyên Mục / Hệ Thống / Cá Nhân / Liên Kết Nhanh)
- **Mega Menu Desktop** — dropdown "🧭 Khám Phá" 3 cột với 18 link, fix lỗi route mồ côi trên desktop
- **Mobile Drawer đầy đủ** — 6 nhóm với 22 link (vs 4 chuyên mục + Hồ sơ + Thoát ở v0.9.13)
- **Bottom Nav redesign** — icon giữa đổi từ 🪷 sang 🧭 (Tổng Quan); tab cuối đổi từ "Không Gian" sang "🙏 Niệm Phật"
- **Footer 5 cột** — Giới Thiệu + Chuyên Mục + Hệ Thống + Cá Nhân + Khám Phá (vs 4 cột cũ)
- **Trang Cài Đặt `/cai-dat`** — 4 nhóm cài đặt: Riêng Tư, Thông Báo, Giao Diện, Chat & Niệm Phật
- **Migration 017**: bảng `user_settings` (16 cột) + trigger `updated_at` + seed default
- **Mục tiêu**: User có thể truy cập MỌI route chỉ từ UI — không còn "route mồ côi" nào

### Giai đoạn 19: 150 Quyền + Thành Tích + Tìm Kiếm toàn cục ✅ (v0.9.14)
- **Hệ thống 150 quyền chi tiết** — mở rộng từ 50 → 150, thêm 100 quyền mới chia 10 nhóm × 10 quyền:
  - `fund` (10) — Quản lý Quỹ Từ Bi
  - `achievements` (10) — Quản lý Thành Tích
  - `security` (10) — Bảo mật & chống spam
  - `navigation` (10) — Quản lý UI/Navigation/Settings
  - `analytics` (10) — Phân tích & báo cáo
  - `media` (10) — Quản lý media/uploads
  - `friends` (10) — Quản lý Bạn Bè/DM
  - `mail` (10) — Quản lý Thư/Thông báo
  - `events` (10) — Quản lý Sự kiện/Cộng tu
  - `shop` (10) — Quản lý Thương Thành
- **Phân bổ mới**: admin_ky_thuat (150) > admin_quan_li (100) > admin_cong_dong (75) > member (0)
- **Hệ thống Thành Tích** — 30 achievements mẫu chia 6 nhóm (Niệm Phật, Tượng Phật, Cộng Đồng, Kinh Sách, Bạn Bè, Quỹ Từ Bi) với 5 độ hiếm (common → mythic)
- **Trang `/thanh-tich`** — cards thành tích đã đạt + progress bar cho đang tiến hành + 5 stats tổng quan
- **Trang `/tim-kiem`** — search đồng thời users + books + topics + groups, tối đa 10 kết quả mỗi loại
- **Migration 018**: 100 quyền mới + role_permissions (325 row, idempotent)
- **Migration 019**: 3 bảng (`achievements`, `user_achievements`, `achievement_progress`) + 2 view + function `check_and_grant_achievement()`
- **Mục tiêu**: Hệ thống phân quyền chi tiết 150 quyền, thành tích động viên user tu tập, tìm kiếm toàn cục

---

## Cấu Trúc Dự Án (Giai đoạn 12 / v0.9.8)

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
│   │   ├── mod.rs           # Page handlers + session auth + profile update + placeholder pages
│   │   ├── auth.rs          # google_login, google_callback, logout (POST-only)
│   │   ├── chat.rs          # Live Chat WebSocket + chat-history REST + ChatHub + GlobalChatHub
│   │   ├── community.rs     # Groups + Topics + Comments handlers + group cover upload
│   │   ├── kinh_sach.rs     # [v0.9.6] Kinh Sách (Books + Chapters + Reviews + Flowers)
│   │   ├── friends.rs       # [v0.9.5] Friends + DM + Mail + Notifications
│   │   ├── uploads.rs       # Upload ảnh API + change avatar (5MB max, SHA-256)
│   │   └── admin.rs         # [v0.9.7] Admin panel: dashboard + user list + role management
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs          # User, GoogleUserInfo, MemberRank, ProfileUpdate + role helpers (v0.9.7)
│   │   ├── community.rs     # Group, Topic, Comment, GroupMember, GroupCategory, ChatMessage
│   │   ├── friends.rs       # [v0.9.5] Friendship, Conversation, DirectMessage, Mail, Notification
│   │   └── kinh_sach.rs     # [v0.9.6] Book, BookChapter, BookReview, BookCategory
│   └── static/
│       ├── css/app.css
│       ├── js/app.js        # + liveChat() + globalChat() Alpine.js components
│       └── uploads/         # Nơi lưu ảnh user upload
├── templates/                # Askama templates (Vietnamese)
│   ├── layout.html
│   ├── home.html
│   ├── profile.html
│   ├── auth/
│   │   └── login.html
│   ├── admin/                # [v0.9.7] Admin panel (dashboard + user list + role UI)
│   │   ├── index.html
│   │   └── users.html
│   ├── community/            # Cộng Đồng (groups + topics + comments + live chat)
│   │   ├── index.html
│   │   ├── group.html
│   │   ├── topic.html
│   │   ├── create_group.html
│   │   └── create_topic.html
│   └── kinh-sach/            # [v0.9.6] Kinh Sách (books + chapters + reviews)
│       ├── index.html
│       ├── category.html
│       ├── search.html
│       ├── book.html
│       └── chapter.html
├── migrations/                # 13 migration files
│   ├── 001_create_users_sessions.sql
│   ├── 002_google_oauth.sql
│   ├── 003_member_profile_ranks.sql
│   ├── 004_storage_images_audit.sql
│   ├── 005_community_groups_topics_comments.sql
│   ├── 006_group_chat_messages.sql
│   ├── 007_global_chat_messages.sql
│   ├── 008_friendships.sql
│   ├── 009_conversations_direct_messages.sql
│   ├── 010_mails.sql
│   ├── 011_notifications.sql
│   ├── 012_kinh_sach.sql
│   └── 013_admin_roles.sql      # [v0.9.7] Hệ thống vai trò Admin (member/admin_ky_thuat/admin_cong_dong/admin_quan_li)
├── .github/workflows/
│   └── docker.yml            # [v0.9.4] Build & push GHCR (ghcr.io/mhieuhonda/tubi-app) + trigger Coolify API
├── HieuLouis/                # Tài liệu dự án
├── Cargo.toml                # v0.9.7, Rust 1.97, axum ws feature, release profile tối ưu
├── Cargo.lock                # Lock file (commit cho reproducible build)
├── Dockerfile                # Multi-stage Rust 1.97.1, ~30 MB
├── docker-compose.yml        # Dev environment (Postgres 17 + app)
├── .dockerignore             # Tối ưu build context
├── .env.example              # Template cấu hình môi trường v0.9
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

### Production (CI/CD tự động qua GitHub Actions + Coolify)

Từ v0.9.4, dự án áp dụng mô hình CI/CD hoàn toàn tự động:

1. **Push code** lên branch `main` (hoặc tạo tag `v*`)
2. **GitHub Actions** tự động:
   - Build Docker image với Rust 1.97.1 (multi-stage, image final ~30 MB)
   - Push lên GHCR: `ghcr.io/mhieuhonda/tubi-app:{latest,sha-<short>,vX.Y.Z}`
   - Gọi Coolify API `/api/v1/applications/{uuid}/start` để trigger deploy
3. **Coolify** tự động:
   - Pull image `:latest` từ GHCR
   - Stop container cũ, start container mới
   - Run health check trên `https://tubi.louis.vangioitutien.com/api/health`
   - Cấp phát SSL tự động qua Traefik + Let's Encrypt (đã có sẵn)
4. Kiểm tra trạng thái: `https://tubi.louis.vangioitutien.com/api/health`

**Cấu hình đã có trên Coolify:**
- Sub VPS: `10.187.247.3` (Ubuntu 24.04, 6 CPU, 8 GB RAM) — server `vangioi-vps`
- PostgreSQL 17 chạy riêng trên sub VPS (DATABASE_URL set trong env của app)
- Domain: `https://tubi.louis.vangioitutien.com` (Traefik reverse proxy + Let's Encrypt)
- Healthcheck: `GET /api/health` (port 8080, scheme http, retries 5, start-period 60s)
- Volume bind: `/app/static/uploads` để giữ ảnh user upload giữa các lần deploy
- Auto-migrations: chạy khi khởi động (`RUN_MIGRATIONS=true`, `APP_ENV=production`)
- Sentinel: bật (giám sát VPS, push metrics mỗi 60s, giữ history 7 ngày)

**GitHub Secrets cần thiết:**
- `COOLIFY_API_TOKEN` — API token của Coolify (tạo ở User Settings → API Tokens)
- `COOLIFY_APP_UUID` — UUID của application trên Coolify

**Rollback:** Đổi tag image trong Coolify từ `:latest` sang `:sha-<old>` hoặc `:v0.9.3` → deploy lại.

**Lịch sử thay đổi CI/CD:**
- v0.5 — GitHub Actions đầu tiên (build + push GHCR + webhook)
- v0.9.1 — Bỏ GitHub Actions, chuyển sang deploy thủ công qua Coolify (build từ source trên VPS)
- v0.9.4 — Quay lại GitHub Actions nhưng với mô hình Docker Image (Coolify pull image, không build từ source)

## Routes (v0.9.7)

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
| GET | `/cong-dong/nhom/{slug}` | **[v0.6]** Trang nhóm + danh sách chủ đề + Live Chat panel | Public |
| POST | `/cong-dong/nhom/{slug}/tham-gia` | **[v0.6]** Tham gia nhóm | Auth |
| POST | `/cong-dong/nhom/{slug}/roi-khoi` | **[v0.6]** Rời nhóm (owner không được rời) | Auth |
| GET | `/cong-dong/nhom/{slug}/tao-chu-de` | **[v0.6]** Form tạo chủ đề | Auth + member |
| POST | `/cong-dong/nhom/{slug}/tao-chu-de` | **[v0.6]** Tạo chủ đề mới | Auth + member |
| GET | `/cong-dong/chu-de/{id}` | **[v0.6]** Trang chủ đề + bình luận | Public |
| POST | `/cong-dong/chu-de/{id}/binh-luan` | **[v0.6]** Đăng bình luận | Auth |
| **WS** | `/ws/cong-dong/nhom/{slug}` | **[v0.9.2]** Live Chat WebSocket (upgrade) | Auth + member |
| GET | `/api/cong-dong/nhom/{slug}/chat-history` | **[v0.9.2]** Lấy 50 tin nhắn gần nhất (`?limit=&before=`) | Public |
| GET | `/khong-gian` | Không Gian (placeholder) | Public |
| GET | `/cong-dong` | Cộng Đồng — Lướt Nhóm / Lướt Chủ Đề | Public |
| GET | `/ban-be` | Bạn Bè — Friend list, DM, Mail, Notifications | Public |
| GET | `/kinh-sach` | **[v0.9.6]** Trang chính Kinh Sách (Featured / Popular / Recent) | Public |
| GET | `/kinh-sach/tim-kiem` | **[v0.9.6]** Tìm kiếm sách `?q=` | Public |
| GET | `/kinh-sach/thu-vien/{category_slug}` | **[v0.9.6]** Lọc theo thư viện (phat-gia/dao-gia/kinh-van/sach-quy/quan-trong) | Public |
| GET | `/kinh-sach/{slug}` | **[v0.9.6]** Trang sách + danh sách chương + cảm ngộ | Public |
| GET | `/kinh-sach/{slug}/chuong/{chapter_slug}` | **[v0.9.6]** Đọc chương (sidebar mục lục) | Public |
| POST | `/kinh-sach/{slug}/cam-ngo` | **[v0.9.6]** Gửi cảm ngộ (min 100 chữ, chờ duyệt) | Auth |
| POST | `/kinh-sach/{slug}/tang-hoa` | **[v0.9.6]** Tặng hoa (1 user/sách) | Auth |
| GET | `/admin` | **[v0.9.7]** Trang Quản Trị (dashboard + stats) | Auth + admin |
| GET | `/admin/thanh-vien` | **[v0.9.7]** Danh sách thành viên + role | Auth + admin |
| POST | `/admin/thanh-vien/{user_id}/role` | **[v0.9.7]** Đổi role user (chỉ Admin Quản Lý) | Auth + admin_quan_li |
| GET | `/quy-tu-bi` | **[v0.9.11]** Trang Quỹ Từ Bi (dashboard + form + lists) | Public |
| POST | `/quy-tu-bi/dong-gop` | **[v0.9.11]** Đóng góp K vào quỹ | Auth |
| GET | `/api/quy-tu-bi/stats` | **[v0.9.11]** JSON tổng quan Quỹ Từ Bi | Public |
| GET | `/thuong-thanh` | Thương Thành (placeholder) | Public |
| GET | `/bang-xep-hang` | **[v0.9.10]** Trang Bảng Xếp Hạng (5 tabs) | Public |
| GET | `/tong-quan` | **[v0.9.14]** Trang Tổng Quan (User Hub) — 24 tính năng | Public |
| GET | `/cai-dat` | **[v0.9.14]** Trang Cài Đặt cá nhân | Auth |
| POST | `/cai-dat/cap-nhat` | **[v0.9.14]** Cập nhật cài đặt | Auth |
| GET | `/thanh-tich` | **[v0.9.14]** Trang Thành Tích cá nhân | Auth |
| GET | `/api/thanh-tich/stats` | **[v0.9.14]** JSON tổng quan thành tích | Auth |
| GET | `/tim-kiem` | **[v0.9.14]** Tìm kiếm toàn cục (?q=) | Public |
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
- **v0.9** — Giai đoạn 9: Codebase sạch lỗi, clippy pedantic/nursery pass, Axum 0.8 ổn định
- **v0.9.1** — Fix UI mobile (bottom nav + x-cloak), bỏ GitHub Actions, deploy thủ công qua Coolify
- **v0.9.2** — Giai đoạn 7: Live Chat WebSocket trong Nhóm (Axum 0.8 ws + ChatHub broadcast + Alpine.js liveChat component)
- **v0.9.3** — Fix live chat (mpsc channel + tokio::select!), thêm Chat Chung toàn platform, avatar/group image upload, favicon
- **v0.9.4** — Giai đoạn 8: CI/CD tự động (GitHub Actions build & push Docker Image lên GHCR → Coolify auto pull & deploy)
- **v0.9.5** — Giai đoạn 9: Module Bạn Bè (Friends + DM 1-1 WebSocket + Mail + Notifications) + Fix live chat bugs (HttpOnly cookie check)
- **v0.9.6** — Giai đoạn 10: Kinh Sách (Thư viện kinh sách Phật giáo & Đạo giáo — 5 thư viện + Books + Chapters + Reviews + Flowers + Search)
- **v0.9.7** — Giai đoạn 11: Hệ thống vai trò Admin & Phân quyền cộng đồng (4 roles + admin panel + role display trên profile/header)
- **v0.9.8** — Giai đoạn 12: 50 quyền chi tiết + 3 giao diện admin riêng biệt (Admin Kỹ Thuật 50 quyền cao nhất + terminal/mod/executive dashboards)
- **v0.9.9** — Giai đoạn 13: Không Gian Cá Nhân (Niệm Phật Counter + Tượng Phật vows + Nhật Ký Tu Học + i_balance)
- **v0.9.10** — Giai đoạn 14: Bảng Xếp Hạng (5 tabs: A/I/K/Hôm Nay/Streak) + Safety schema fix cho login
- **v0.9.11** — Giai đoạn 15: Quỹ Từ Bi (đóng góp K + dashboard + 5 loại quỹ) + Fix Docker cache stale (CRITICAL deploy fix cho login)
- **v0.9.14** — Giai đoạn 18 + 19: Navigation Overhaul (User Hub + Mega Menu + Mobile Drawer + Settings) + 150 Quyền chi tiết + Hệ thống Thành Tích + Tìm Kiếm toàn cục

---

*Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.* 🪷
