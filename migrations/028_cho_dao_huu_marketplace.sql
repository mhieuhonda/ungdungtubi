-- =====================================================================
-- Migration 028 — Giai đoạn 44: Chợ Đạo Hữu + Admin Hoàn Thiện
-- v0.9.40 — 2026-08-17
--
-- Mục tiêu:
--   Theo yêu cầu user: "xóa hết phần 'Đăng Bán Vật Phẩm PvP' vì game đã
--   bị xóa hoàn toàn và thay bằng một loại đăng bán khác mà người đăng có
--   thể chọn các danh mục tùy ý, chọn có sẵn trong hệ thống hoặc tạo mới.
--   Và khi đăng vật phẩm thì có thể chọn nhận tiền bằng K hoặc bằng ngân
--   hàng (tự điền thông tin)."
--
--   Đồng thời hoàn thiện các bảng quản trị (admin) đang là placeholder.
--
-- Thay đổi:
--   1. Tạo bảng `shop_categories` — danh mục Thương Thành do admin + user
--      tạo. User khi đăng bán có thể chọn danh mục có sẵn hoặc tạo mới.
--   2. Thêm cột mới vào `shop_items`:
--      - `category_id`     — link tới shop_categories(id) (nullable cho
--                            back-compat với các row cũ dùng `category` TEXT).
--      - `payment_method`  — 'k' (nhận K) hoặc 'bank' (nhận chuyển khoản).
--      - `price_vnd`       — giá VNĐ khi payment_method = 'bank'.
--      - `bank_info`       — JSONB {bank_name, account_number, account_holder,
--                            qr_image_url} khi payment_method = 'bank'.
--      - `is_featured`     — admin có thể set nổi bật.
--      - `moderation_status` — 'pending' | 'approved' | 'rejected' | 'removed'.
--   3. Thêm cột mới vào `transactions`:
--      - `payment_method`  — 'k' hoặc 'bank'.
--      - `price_vnd`       — snapshot giá VNĐ lúc giao dịch.
--      - `bank_info`       — snapshot bank_info lúc giao dịch.
--      - `buyer_contact`   — thông tin liên hệ người mua (khi bank transfer).
--
-- Idempotent: tất cả CREATE TABLE IF NOT EXISTS / ADD COLUMN IF NOT EXISTS.
-- =====================================================================

-- ════════════════════════════════════════════════════════════════════════════
-- 1. Bảng shop_categories — Danh mục Thương Thành
-- ════════════════════════════════════════════════════════════════════════════
-- Danh mục có thể do admin tạo (is_system = true) hoặc do user tự tạo khi
-- đăng bán (is_system = false, created_by = user_id). User-created categories
-- cần admin duyệt trước khi xuất hiện công khai (is_approved).
CREATE TABLE IF NOT EXISTS shop_categories (
    id              BIGSERIAL    PRIMARY KEY,
    -- Slug URL-friendly: VD "the-tu-hoc", "vat-phham-phat-giao"
    slug            TEXT         NOT NULL UNIQUE,
    -- Tên hiển thị tiếng Việt: VD "Thẻ Tu Học", "Vật Phẩm Phật Giáo"
    name_vi         TEXT         NOT NULL,
    -- Mô tả danh mục
    description     TEXT,
    -- Biểu tượng emoji
    icon            TEXT         NOT NULL DEFAULT '📦',
    -- Màu sắc (hex)
    color           TEXT         NOT NULL DEFAULT '#0F766E',
    -- Danh mục cha (NULL = top-level)
    parent_id       BIGINT       REFERENCES shop_categories(id) ON DELETE SET NULL,
    -- Thứ tự sắp xếp
    sort_order      INTEGER      NOT NULL DEFAULT 0,
    -- System category (do admin tạo) vs user-submitted
    is_system       BOOLEAN      NOT NULL DEFAULT false,
    -- Admin đã duyệt category (user-submitted cần duyệt trước khi public)
    is_approved     BOOLEAN      NOT NULL DEFAULT true,
    -- Hiển thị công khai
    is_active       BOOLEAN      NOT NULL DEFAULT true,
    -- Người tạo (NULL = system seed)
    created_by      UUID         REFERENCES users(id) ON DELETE SET NULL,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_shop_categories_parent
    ON shop_categories (parent_id, sort_order) WHERE is_active = true;

CREATE INDEX IF NOT EXISTS idx_shop_categories_active
    ON shop_categories (is_active, is_approved);

-- Seed các danh mục hệ thống (is_system = true) — đồng bộ với category TEXT
-- cũ trong shop_items để đảm bảo backward compatibility.
INSERT INTO shop_categories (slug, name_vi, description, icon, color, sort_order, is_system, is_approved)
VALUES
    ('the-tu-hoc',      'Thẻ Tu Học',       'Các thẻ hỗ trợ tu học: Tự Tu, Cộng Tu, Exp.',                    '📿', '#2E7D32', 1,  true, true),
    ('the-doi-ten',     'Thẻ Đổi Tên',      'Thẻ đổi tên, pháp danh, pháp hiệu.',                            '✏️', '#6A1B9A', 2,  true, true),
    ('the-ho-tro',      'Thẻ Hỗ Trợ',       'Thẻ hỗ trợ cộng đồng, ủng hộ quỹ, hộp quà.',                    '🤝', '#C62828', 3,  true, true),
    ('the-nhom',        'Thẻ Nhóm',         'Thẻ tạo nhóm, không gian nhóm, mời cộng tu.',                    '👥', '#3F51B5', 4,  true, true),
    ('the-bau-chon',    'Thẻ Bầu Chọn',     'Thẻ tạo cuộc bầu chọn trong nhóm/cộng đồng.',                   '🗳️', '#673AB7', 5,  true, true),
    ('vat-pham',        'Vật Phẩm',         'Vật phẩm chung: hoa hồng, ô vật phẩm, thẻ yêu cầu.',            '📦', '#795548', 6,  true, true),
    ('cao-cap',         'Cao Cấp',          'Vật phẩm cao cấp: Phiếu Từ Bi, Thẻ Người Tốt, Thẻ Thiện Nhân.', '🪷', '#0F766E', 7,  true, true),
    ('sach-phat-giao',  'Sách Phật Giáo',   'Sách điện tử, kinh sách do đạo hữu chia sẻ.',                   '📚', '#FF6F00', 8,  true, true),
    ('do-tho',          'Đồ Thờ',           'Đồ thờ cúng: tượng Phật, hoa sen, đèn nến.',                    '🪔', '#FFD600', 9,  true, true),
    ('dich-vu',         'Dịch Vụ',          'Dịch vụ Phật giáo: in kinh, tổ chức lễ, hướng dẫn tu.',         '🛎️', '#0288D1', 10, true, true),
    ('thuc-pham-chay',  'Thực Phẩm Chay',   'Thực phẩm chay, đồ素食 hữu cơ.',                                 '🥬', '#43A047', 11, true, true),
    ('khac',            'Khác',             'Danh mục khác — không thuộc nhóm nào trên.',                     '🏷️', '#607D8B', 99, true, true)
ON CONFLICT (slug) DO NOTHING;

-- Backfill category_id cho shop_items cũ (dựa trên category TEXT → slug)
-- Dùng UPDATE ... FROM để map an toàn (idempotent).
UPDATE shop_items si
SET category_id = sc.id, updated_at = NOW()
FROM shop_categories sc
WHERE si.category_id IS NULL
  AND si.category IS NOT NULL
  AND sc.slug = REPLACE(LOWER(si.category), '_', '-');

-- ════════════════════════════════════════════════════════════════════════════
-- 2. Thêm cột mới vào shop_items
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS category_id BIGINT REFERENCES shop_categories(id) ON DELETE SET NULL;
ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS payment_method TEXT NOT NULL DEFAULT 'k' CHECK (payment_method IN ('k', 'bank'));
ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS price_vnd BIGINT CHECK (price_vnd IS NULL OR price_vnd >= 0);
ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS bank_info JSONB DEFAULT '{}';
ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE shop_items ADD COLUMN IF NOT EXISTS moderation_status TEXT NOT NULL DEFAULT 'approved'
    CHECK (moderation_status IN ('pending', 'approved', 'rejected', 'removed'));

-- Update CHECK constraint cho store: thêm 'dao_huu' (Chợ Đạo Hữu) như alias
-- cho 'pvp' để giữ backward compat. Cột store vẫn giữ 'pvp' cho data cũ;
-- các đăng bán mới từ v0.9.40 sẽ dùng 'dao_huu'. Cả 2 đều render như "Chợ Đạo Hữu".
-- (PostgreSQL không cho ADD CHECK mới trùng tên — tạo constraint mới.)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'shop_items_store_check_v2'
    ) THEN
        ALTER TABLE shop_items ADD CONSTRAINT shop_items_store_check_v2
            CHECK (store IN ('app', 'pvp', 'dao_huu'));
    END IF;
END$$;

-- Index cho moderation queue (admin duyệt)
CREATE INDEX IF NOT EXISTS idx_shop_items_moderation
    ON shop_items (moderation_status, created_at DESC)
    WHERE store IN ('pvp', 'dao_huu');

-- Index cho category_id
CREATE INDEX IF NOT EXISTS idx_shop_items_category_id
    ON shop_items (category_id)
    WHERE category_id IS NOT NULL;

-- Index cho featured
CREATE INDEX IF NOT EXISTS idx_shop_items_featured
    ON shop_items (is_featured, sort_order)
    WHERE is_active = true AND is_featured = true;

-- ════════════════════════════════════════════════════════════════════════════
-- 3. Thêm cột mới vào transactions
-- ════════════════════════════════════════════════════════════════════════════
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS payment_method TEXT NOT NULL DEFAULT 'k' CHECK (payment_method IN ('k', 'bank'));
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS price_vnd BIGINT CHECK (price_vnd IS NULL OR price_vnd >= 0);
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS bank_info JSONB DEFAULT '{}';
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS buyer_contact TEXT;

-- Update tx_type CHECK: thêm 'bank_transfer' cho giao dịch chuyển khoản ngân hàng.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'transactions_tx_type_check_v2'
    ) THEN
        ALTER TABLE transactions ADD CONSTRAINT transactions_tx_type_check_v2
            CHECK (tx_type IN ('purchase', 'sale', 'transfer', 'refund', 'bank_transfer'));
    END IF;
END$$;

-- ════════════════════════════════════════════════════════════════════════════
-- 4. Comments
-- ════════════════════════════════════════════════════════════════════════════
COMMENT ON TABLE shop_categories IS
    'v0.9.40 — Danh mục Thương Thành. System (admin tạo) hoặc user-submitted (cần duyệt).';
COMMENT ON COLUMN shop_items.payment_method IS
    'v0.9.40 — Phương thức thanh toán: k (nhận K) hoặc bank (nhận chuyển khoản ngân hàng).';
COMMENT ON COLUMN shop_items.bank_info IS
    'v0.9.40 — JSONB {bank_name, account_number, account_holder, qr_image_url} khi payment_method=bank.';
COMMENT ON COLUMN shop_items.moderation_status IS
    'v0.9.40 — Trạng thái kiểm duyệt: pending (chờ duyệt), approved (đã duyệt), rejected (từ chối), removed (bị gỡ).';
COMMENT ON COLUMN transactions.payment_method IS
    'v0.9.40 — Snapshot payment_method lúc giao dịch.';
COMMENT ON COLUMN transactions.bank_info IS
    'v0.9.40 — Snapshot bank_info người bán lúc giao dịch (tránh mất dữ liệu khi seller sửa).';
