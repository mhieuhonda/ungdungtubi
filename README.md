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
| Template | Askama (type-safe HTML templates) |
| Database | PostgreSQL + SQLx (async, compile-time checked) |
| Frontend | HTMX (server-driven UI) + Alpine.js (reactive) |
| Styling | Tailwind CSS |

## 4 Chuyên Mục Chính

1. 🌍 **Không Gian** – Không gian cá nhân, cộng tu, niệm Phật
2. 👥 **Cộng Đồng** – Diễn đàn, nhóm, chủ đề, live chat
3. 👤 **Bạn Bè** – Kết nối, nhắn tin, gửi thư
4. 📚 **Kinh Sách** – Thư viện kinh sách Phật giáo & Đạo giáo

---

## Lộ Trình 25 Giai Đoạn Phát Triển

### Giai đoạn 1: Kiến tạo nền móng — Thiết lập dự án & hạ tầng cốt lõi
- Khởi tạo project Rust (Actix-web + Askama + SQLx + PostgreSQL)
- Cấu hình HTMX + Alpine.js + Tailwind CSS
- Thiết kế database schema nền tảng (users, sessions)
- Trang landing page / trang chủ
- Hệ thống template layout (header, footer, navigation)
- Cấu hình domain `tubi.louis.vangioitutien.com`
- **Mục tiêu:** Server chạy được, hiển thị trang chủ với giao diện cơ bản

### Giai đoạn 2: Hệ thống xác thực — Đăng ký & Đăng nhập
- Form đăng ký thành viên (email, mật khẩu, tên hiển thị)
- Đăng nhập (email + password)
- Session management (cookie-based, SQLx session store)
- Logout & bảo vệ route
- Migrate database: bảng `users`, `sessions`
- **Mục tiêu:** Thành viên có thể đăng ký, đăng nhập, đăng xuất

### Giai đoạn 3: Hồ sơ thành viên & Hệ thống cấp bậc
- Trang hồ sơ cá nhân (tên, pháp danh, pháp hiệu, bút danh, giới tính)
- Chỉnh sửa hồ sơ
- Hệ thống cấp bậc: Người Mới → Người Thường → ... → Thiện Nhân → Đại Gia
- Hiển thị cấp bậc trên profile và header
- Migrate: bảng `member_ranks`
- **Mục tiêu:** Hồ sơ hoạt động, cấp bậc hiển thị đúng

### Giai đoạn 4: Không Gian — Trang cá nhân & nhân vật
- Giao diện Không Gian (trang mặc định sau đăng nhập)
- Nhân vật quả cầu ánh sáng (CSS/SVG animation)
- Điều khiển nhân vật di chuyển (Alpine.js reactive)
- Migrate: bảng `spaces`, `space_buildings`
- **Mục tiêu:** Không Gian hiển thị với nhân vật có thể di chuyển

### Giai đoạn 5: Nút Niệm — Hệ thống đếm niệm Phật
- Nút Niệm (N) – đếm số lần niệm "A Di Đà Phật"
- Chạy nền khi tắt màn hình (service worker / timer)
- Quy đổi: 1 lần niệm = 1 A, 1000 A = 1 K
- Hiển thị thanh A và K trên UI
- Migrate: bảng `prayer_sessions`, `prayer_counts`
- **Mục tiêu:** Niệm Phật đếm được, A/K cập nhật realtime

### Giai đoạn 6: Thanh A & Thanh I — Kỹ năng cốt lõi
- Thanh A: kích hoạt niệm → sóng âm lan tỏa (hiệu ứng CSS)
- Thanh I: trạng thái thiền định, bất khả xâm phạm
- Thời gian hồi chiêu & vận công
- Thanh A hủy Thanh I, thanh I bất khả tác động
- Migrate: bảng `skill_states`
- **Mục tiêu:** Hai kỹ năng cốt lõi hoạt động đúng logic

### Giai đoạn 7: Nhà Nhật Ký — Ghi chép hoạt động
- Nhật Ký Hệ Thống (tự động ghi lại hoạt động)
- Nhật Ký Thành Viên (đăng bài, chia sẻ — giống Facebook wall)
- Chế độ công khai / riêng tư
- Bình luận (nếu cho phép)
- Migrate: bảng `journals`, `journal_entries`, `journal_comments`
- **Mục tiêu:** Nhật ký hoạt động, đăng bài, bình luận

### Giai đoạn 8: Nhà Nhạc — Quản lý & phát nhạc
- 5 thư mục: Niệm, Thiền, Đạo, Không Lời, Cá Nhân
- Phát nhạc (HTML5 Audio API)
- Chế độ: lặp, ngẫu nhiên, hẹn giờ tắt
- Mọi người trong Không Gian cùng nghe
- Migrate: bảng `music_tracks`, `music_playlists`
- **Mục tiêu:** Nhạc phát được, playlist quản lý được

### Giai đoạn 9: Nhà Cài Đặt — Cài đặt hệ thống
- Cài đặt hồ sơ, quyền riêng tư
- Bật/tắt chức năng
- Cài đặt ngôn ngữ (VI/EN/CN)
- Cài đặt thông báo
- Migrate: bảng `user_settings`
- **Mục tiêu:** Cài đặt cá nhân lưu trữ và áp dụng đúng

### Giai đoạn 10: Thùng Từ Bi & Thương Thành nền tảng
- Rương Chứa Đồ, Rương Tặng Đồ, Rương Bán Đồ
- Rương Từ Bi (quyên góp, phát quà)
- Giao diện Thương Thành cơ bản
- Migrate: bảng `inventory`, `marketplace_items`, `charity_fund`
- **Mục tiêu:** Hệ thống vật phẩm & quỹ từ bi hiển thị

### Giai đoạn 11: Hệ thống tiền tệ — A, K, Phiếu Từ Bi
- Quy đổi: 1000 A = 1 K
- Phiếu Từ Bi (mua bằng K, sau 100 ngày → Bi)
- Lịch sử giao dịch
- Phí giao dịch (tối thiểu 10%)
- Migrate: bảng `wallets`, `transactions`, `vouchers`
- **Mục tiêu:** Tiền tệ hoạt động, giao dịch ghi nhận đúng

### Giai đoạn 12: Cộng Đồng — Nền tảng nhóm & chủ đề
- Danh sách nhóm, tạo nhóm, tham gia nhóm
- Chủ đề trong nhóm, tạo chủ đề
- Lướt nhóm (giao diện feed giống TikTok/Facebook)
- Migrate: bảng `groups`, `group_members`, `topics`, `topic_posts`
- **Mục tiêu:** Nhóm & chủ đề CRUD, lướt được

### Giai đoạn 13: Cộng Đồng — Bình luận & Live Chat
- Bình luận trong chủ đề (threaded comments)
- Live chat trong nhóm (WebSocket/HTMX polling)
- Hiển thị chủ đề mới nhất, nổi bật
- Migrate: bảng `comments`, `chat_messages`
- **Mục tiêu:** Thảo luận realtime trong nhóm

### Giai đoạn 14: Cộng Đồng — Bầu chọn & Không Gian nhóm
- Tạo cuộc bầu chọn (đề xuất, bỏ phiếu)
- Không Gian Chung cho nhóm (tương tự cá nhân nhưng cho nhóm)
- Thẻ Tạo Không Gian Nhóm
- Migrate: bảng `polls`, `poll_votes`, `group_spaces`
- **Mục tiêu:** Bầu chọn & không gian nhóm hoạt động

### Giai đoạn 15: Bạn Bè — Kết bạn & danh sách
- Tìm kiếm theo ID / tên
- Gửi lời mời kết bạn, chấp nhận, từ chối
- Danh sách bạn bè, danh sách chặn
- Hiển thị trạng thái online
- Migrate: bảng `friendships`, `friend_requests`, `blocked_users`
- **Mục tiêu:** Kết bạn đầy đủ CRUD

### Giai đoạn 16: Bạn Bè — Nhắn tin & Gửi thư
- Nhắn tin realtime (WebSocket/HTMX)
- Gửi thư (giống email nội bộ)
- Lời mời cộng tu, mời chơi qua tin nhắn
- Migrate: bảng `messages`, `letters`
- **Mục tiêu:** Chat & thư nội bộ hoạt động

### Giai đoạn 17: Kinh Sách — Thư viện & phân loại
- 5 thư viện: Phật Gia, Đạo Gia, Kinh Văn, Sách Quý, Quan Trọng
- Phân loại ngôn ngữ (VI/EN/CN)
- Tìm kiếm sách (tên, tác giả, từ khóa)
- Giao diện danh sách sách (theo mockup)
- Migrate: bảng `books`, `book_categories`, `book_tags`
- **Mục tiêu:** Thư viện hiển thị, lọc, tìm kiếm

### Giai đoạn 18: Kinh Sách — Đọc sách & tải xuống
- Đọc sách trực tuyến (PDF/EPUB viewer)
- Tải sách ngoại tuyến
- Viết cảm ngộ (tối thiểu 100 chữ, xét duyệt)
- Donate / tặng hoa / hỗ trợ tác giả
- Migrate: bảng `book_reviews`, `donations`
- **Mục tiêu:** Đọc & tải sách hoạt động

### Giai đoạn 19: Hệ thống thành tích & Bảng xếp hạng
- Thống kê niệm Phật (ngày/tuần/tháng/năm/tổng)
- BXH: Niệm Phật, Tài Phú K, Niệm Lực A, Phiếu Từ Bi, Từ Bi
- Quy đổi điểm Từ Bi
- Migrate: bảng `achievements`, `leaderboards`
- **Mục tiêu:** Thành tích & BXH hiển thị chính xác

### Giai đoạn 20: Hệ thống quản lý — Admin & Mod
- Trang quản trị riêng
- Phân quyền: Admin Kỹ Thuật, Admin Phát Triển, Admin Quản Lý, Mod, Min
- Quản lý thành viên, nội dung, nhóm
- Migrate: bảng `admin_roles`, `admin_actions`
- **Mục tiêu:** Quản trị hoạt động, phân quyền đúng

### Giai đoạn 21: Cửa Hàng Ứng Dụng — Thẻ & vật phẩm
- Danh sách sản phẩm (Ô Vật Phẩm, Thẻ Tự Tu, Thẻ Cộng Tu, v.v.)
- Mua bằng K, thêm vào inventory
- Sử dụng thẻ (hiệu ứng tương ứng)
- Migrate: bảng `shop_items`, `purchases`, `item_effects`
- **Mục tiêu:** Cửa hàng CRUD, mua & sử dụng

### Giai đoạn 22: Game Siêu Độ — Bản đồ & di chuyển
- Hệ thống bản đồ (hình vuông, 4 cổng)
- 10 bản đồ = 1 khu vực, cấp độ bản đồ
- Di chuyển qua cổng / truyền tống (tiêu hao A)
- Giao diện bản đồ (canvas/HTML)
- Migrate: bảng `game_maps`, `player_positions`
- **Mục tiêu:** Di chuyển trên bản đồ hoạt động

### Giai đoạn 23: Game Siêu Độ — Chỉ số & chiến đấu
- Chỉ số A (ATK) & I (HP), hộ thuẫn
- Cơ chế tổn thương, hồi phục
- Siêu độ quái vật bằng niệm Phật (chỉ niệm mới ra A)
- Kỹ năng phụ (4 ô: cơ bản, chủ động, bị động, tuyệt kỹ)
- Migrate: bảng `player_stats`, `monsters`, `skills`
- **Mục tiêu:** Chiến đấu cơ bản hoạt động

### Giai đoạn 24: Game Siêu Độ — PK & bẫy
- Điểm PK, các mốc PK (10, 100, 1000, 10000, 100000, 1000000)
- Hệ thống bẫy (hệ thống, người chơi tạo, quảng cáo)
- Bảng xếp hạng chiến tích A trong bản đồ (reset mỗi giờ)
- Migrate: bảng `pk_records`, `traps`, `map_rankings`
- **Mục tiêu:** PK & bẫy hoạt động theo quy tắc

### Giai đoạn 25: Toàn cầu hóa & Triển khai Production
- Đa ngôn ngữ hoàn chỉnh (i18n VI/EN/CN)
- AI dịch nội dung kinh sách
- Docker + Docker Compose (Rust + PostgreSQL + Nginx)
- CI/CD pipeline
- Monitoring & logging
- SSL/TLS cho `tubi.louis.vangioitutien.com`
- Performance optimization & load testing
- **Mục tiêu:** Ứng dụng sẵn sàng production

---

## Cấu Trúc Dự Án (Giai đoạn 1)

```
ungdungtubi/
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration
│   ├── db/
│   │   ├── mod.rs
│   │   └── migrations/      # SQLx migrations
│   ├── handlers/            # Route handlers
│   │   ├── mod.rs
│   │   ├── home.rs
│   │   ├── auth.rs
│   │   └── ...
│   ├── models/              # Database models
│   │   ├── mod.rs
│   │   └── user.rs
│   ├── templates/           # Askama templates
│   │   ├── layout.html
│   │   ├── home.html
│   │   ├── auth/
│   │   └── ...
│   └── static/              # CSS, JS, images
│       ├── css/
│       ├── js/
│       └── img/
├── migrations/              # SQL migration files
├── Cargo.toml
├── .env
└── README.md
```

## Phiên Bản

- **v0.1** — Giai đoạn 1: Nền móng hạ tầng cốt lõi

---

*Nguyện công đức vô lượng. Nam Mô A Di Đà Phật.* 🪷
