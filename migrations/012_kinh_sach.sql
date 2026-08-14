-- Ứng Dụng Từ Bi - Migration 012: Kinh Sách (Thư viện kinh sách Phật giáo & Đạo giáo)
-- Giai đoạn 10 (v0.9.6): 1 trong 4 chuyên mục chính của app
--
-- Mục tiêu (theo HieuLouis/Hệ Thống Và Chức Năng Chi Tiết.docx, mục IV. Kinh Sách):
--   * 5 thư viện chính: Phật Gia, Đạo Gia, Kinh Văn, Sách Quý, Quan Trọng
--   * Sách điện tử hoàn chỉnh hoặc theo từng tập (chapters)
--   * Thành viên có thể: đọc online, tải offline
--   * Có thể: Kính (Donate K sau này khi có hệ thống tiền tệ), Tặng hoa, Viết cảm ngộ
--   * Cảm ngộ phải có tối thiểu 100 chữ và qua xét duyệt mới hiển thị
--   * 3 ngôn ngữ: Việt (mặc định), Anh, Trung
--   * Toàn bộ chức năng cơ bản miễn phí

-- 1. Bảng book_categories — 5 thư viện chính
CREATE TABLE IF NOT EXISTS book_categories (
    id          SERIAL       PRIMARY KEY,
    slug        VARCHAR(50)  UNIQUE NOT NULL,
    name        VARCHAR(100) NOT NULL,
    description TEXT,
    icon        VARCHAR(10)  NOT NULL DEFAULT '📚',
    sort_order  INT          NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

INSERT INTO book_categories (slug, name, description, icon, sort_order)
VALUES
    ('phat-gia',   'Phật Gia',   'Kinh sách Phật giáo — kinh điển, luận thư, pháp thoại các vị cao tăng',           '🪷', 1),
    ('dao-gia',    'Đạo Gia',    'Kinh sách Đạo giáo — Đạo Đức Kinh, Nam Hoa Kinh, tham同 đồ thư tịch',             '☯️', 2),
    ('kinh-van',   'Kinh Văn',   'Kinh văn tụng đọc — chú giải, nghi thức, sám pháp, khoa lễ',                     '📜', 3),
    ('sach-quy',   'Sách Quý',    'Sách quý về khoa học, triết học, tâm học, văn học — mở rộng tri thức',          '💎', 4),
    ('quan-trong', 'Quan Trọng', 'Bài viết quan trọng do Quản Lý chọn lựa và đề cử',                                 '⭐', 5)
ON CONFLICT (slug) DO NOTHING;

-- 1.5. Cài đặt pg_trgm extension CHO FULL-TEXT SEARCH (idempotent)
-- Phải chạy TRƯỚC khi tạo GIN index dùng gin_trgm_ops ở phần 2.
-- Dùng DO block với EXCEPTION để migration không fail nếu user DB không có
-- quyền CREATE EXTENSION (rất có thể trên shared/managed PostgreSQL).
-- Nếu pg_trgm không tạo được, app sẽ vẫn hoạt động (ILIKE không cần extension,
-- chỉ không có index nên search chậm hơn).
DO $$
BEGIN
    CREATE EXTENSION IF NOT EXISTS pg_trgm;
EXCEPTION
    WHEN insufficient_privilege OR undefined_file THEN
        RAISE NOTICE 'pg_trgm extension không khả dụng (user không có quyền). App sẽ dùng ILIKE không có index.';
    WHEN OTHERS THEN
        RAISE NOTICE 'Không tạo được pg_trgm: %. App sẽ dùng ILIKE không có index.', SQLERRM;
END $$;

-- 2. Bảng books — Sách điện tử
CREATE TABLE IF NOT EXISTS books (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    slug            VARCHAR(200) UNIQUE NOT NULL,
    title           VARCHAR(300) NOT NULL,
    author          VARCHAR(200),
    translator      VARCHAR(200),
    description     TEXT,
    category_id     INT          REFERENCES book_categories(id) ON DELETE SET NULL,
    -- 'vi' | 'en' | 'zh' — Tiếng Việt mặc định theo HieuLouis/
    language        VARCHAR(10)  NOT NULL DEFAULT 'vi',
    -- Link tới ảnh bìa (URL hoặc path trong /static/uploads)
    cover_url       VARCHAR(500),
    -- Link tải offline (PDF/EPUB) — sẽ bổ sung khi có hệ thống file
    download_url    VARCHAR(500),
    -- 'single' | 'multi' — sách đơn bài hoặc nhiều chương
    book_type       VARCHAR(20)  NOT NULL DEFAULT 'single',
    -- Counters (denormalised, duy trì bằng trigger)
    chapter_count   INT          NOT NULL DEFAULT 0,
    view_count      BIGINT       NOT NULL DEFAULT 0,
    review_count    INT          NOT NULL DEFAULT 0,
    flower_count    INT          NOT NULL DEFAULT 0,
    donation_total_k BIGINT      NOT NULL DEFAULT 0,
    is_featured     BOOLEAN      NOT NULL DEFAULT false,
    -- 'draft' | 'pending_review' | 'published' | 'archived'
    status          VARCHAR(20)  NOT NULL DEFAULT 'published',
    is_active       BOOLEAN      NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_books_category   ON books(category_id);
CREATE INDEX IF NOT EXISTS idx_books_language   ON books(language);
CREATE INDEX IF NOT EXISTS idx_books_status     ON books(status);
CREATE INDEX IF NOT EXISTS idx_books_featured   ON books(is_featured) WHERE is_featured = true;
CREATE INDEX IF NOT EXISTS idx_books_active     ON books(is_active);
CREATE INDEX IF NOT EXISTS idx_books_created    ON books(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_books_view_count ON books(view_count DESC);
-- Full-text search index — chỉ tạo nếu pg_trgm đã được cài (xem phần 1.5)
-- Nếu pg_trgm không có, app vẫn hoạt động với ILIKE (chỉ không có GIN index).
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_trgm') THEN
        CREATE INDEX IF NOT EXISTS idx_books_title_trgm ON books USING gin (title gin_trgm_ops);
    END IF;
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Không tạo được gin_trgm_ops index: %', SQLERRM;
END $$;

-- 3. Bảng book_chapters — Chương mục của sách
CREATE TABLE IF NOT EXISTS book_chapters (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id     UUID         NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    slug        VARCHAR(200) NOT NULL,
    title       VARCHAR(300) NOT NULL,
    content     TEXT         NOT NULL,
    -- Số thứ tự chương (1, 2, 3...)
    sort_order  INT          NOT NULL DEFAULT 0,
    view_count  BIGINT       NOT NULL DEFAULT 0,
    is_active   BOOLEAN      NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE(book_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_book_chapters_book   ON book_chapters(book_id);
CREATE INDEX IF NOT EXISTS idx_book_chapters_sort   ON book_chapters(book_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_book_chapters_active ON book_chapters(is_active);

-- 4. Bảng book_reviews — Cảm ngộ của thành viên về sách
-- Theo HieuLouis/: "Cảm ngộ phải có tối thiểu 100 chữ và qua xét duyệt thì mới được hiển thị."
CREATE TABLE IF NOT EXISTS book_reviews (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id     UUID         NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    user_id     UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    body        TEXT         NOT NULL,
    -- Số hoa tặng (từ user khác) — duy trì bằng trigger
    flower_count INT         NOT NULL DEFAULT 0,
    -- 'pending' | 'approved' | 'rejected' — chỉ hiển thị khi approved
    status      VARCHAR(20)  NOT NULL DEFAULT 'pending',
    is_active   BOOLEAN      NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_book_reviews_book   ON book_reviews(book_id);
CREATE INDEX IF NOT EXISTS idx_book_reviews_user   ON book_reviews(user_id);
CREATE INDEX IF NOT EXISTS idx_book_reviews_status ON book_reviews(status);
CREATE INDEX IF NOT EXISTS idx_book_reviews_active ON book_reviews(is_active);

-- Constraint: một user chỉ được viết 1 cảm ngộ mỗi sách (có thể edit)
CREATE UNIQUE INDEX IF NOT EXISTS uq_book_reviews_book_user ON book_reviews(book_id, user_id) WHERE is_active = true;

-- 5. Bảng book_donations — Kính (Donate K cho sách, sẽ link sang Quỹ Từ Bi sau)
-- Theo HieuLouis/: "Kính (Donate tiền K)" — chưa có hệ thống K chính thức nên
-- đây chỉ là bảng ghi nhận ý định donate, sẽ tự động trừ K khi hệ thống tiền tệ hoàn thiện.
CREATE TABLE IF NOT EXISTS book_donations (
    id          UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    book_id     UUID         NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    user_id     UUID         REFERENCES users(id) ON DELETE SET NULL,
    amount_k    BIGINT       NOT NULL DEFAULT 0,
    message     TEXT,
    is_active   BOOLEAN      NOT NULL DEFAULT true,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_book_donations_book ON book_donations(book_id);
CREATE INDEX IF NOT EXISTS idx_book_donations_user ON book_donations(user_id);

-- 6. Bảng book_flowers — Tặng hoa (1 user/tài khoản có thể tặng hoa 1 lần/sách)
CREATE TABLE IF NOT EXISTS book_flowers (
    id          BIGSERIAL    PRIMARY KEY,
    book_id     UUID         NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    user_id     UUID         NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE(book_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_book_flowers_book ON book_flowers(book_id);
CREATE INDEX IF NOT EXISTS idx_book_flowers_user ON book_flowers(user_id);

-- 7. Triggers: tự cập nhật updated_at (dùng lại hàm trigger_set_updated_at từ migration 004)
DROP TRIGGER IF EXISTS trg_books_set_updated_at ON books;
CREATE TRIGGER trg_books_set_updated_at
    BEFORE UPDATE ON books
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

DROP TRIGGER IF EXISTS trg_book_chapters_set_updated_at ON book_chapters;
CREATE TRIGGER trg_book_chapters_set_updated_at
    BEFORE UPDATE ON book_chapters
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

DROP TRIGGER IF EXISTS trg_book_reviews_set_updated_at ON book_reviews;
CREATE TRIGGER trg_book_reviews_set_updated_at
    BEFORE UPDATE ON book_reviews
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- 8. Triggers: counters
CREATE OR REPLACE FUNCTION update_book_chapter_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE books SET chapter_count = chapter_count + 1 WHERE id = NEW.book_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE books SET chapter_count = GREATEST(chapter_count - 1, 0) WHERE id = OLD.book_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_book_chapters_count ON book_chapters;
CREATE TRIGGER trg_book_chapters_count
    AFTER INSERT OR DELETE ON book_chapters
    FOR EACH ROW EXECUTE FUNCTION update_book_chapter_count();

CREATE OR REPLACE FUNCTION update_book_flower_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE books SET flower_count = flower_count + 1 WHERE id = NEW.book_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE books SET flower_count = GREATEST(flower_count - 1, 0) WHERE id = OLD.book_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_book_flowers_count ON book_flowers;
CREATE TRIGGER trg_book_flowers_count
    AFTER INSERT OR DELETE ON book_flowers
    FOR EACH ROW EXECUTE FUNCTION update_book_flower_count();

CREATE OR REPLACE FUNCTION update_book_review_count()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE books SET review_count = review_count + 1 WHERE id = NEW.book_id;
        RETURN NEW;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE books SET review_count = GREATEST(review_count - 1, 0) WHERE id = OLD.book_id;
        RETURN OLD;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_book_reviews_count ON book_reviews;
CREATE TRIGGER trg_book_reviews_count
    AFTER INSERT OR DELETE ON book_reviews
    FOR EACH ROW EXECUTE FUNCTION update_book_review_count();

-- 9. Seed data: một vài cuốn sách Phật giáo phổ biến để app có sẵn nội dung
-- (Phase 1: thư viện do đội ngũ quản lý xây dựng)
INSERT INTO books (slug, title, author, description, category_id, language, book_type, status, is_featured)
SELECT 'kinh-a-di-qua', 'Kinh A Di Đà', 'Nguyên Thuyết (Trích)', 
       'Kinh A Di Đà (佛說阿彌陀經) là một trong những kinh điển quan trọng nhất của Phật giáo Đại thừa, được Đức Phật Thích Ca Mâu Ni giảng tại vườn Kỳ Thọ Cấp Cô Độc. Kinh mô tả cõi Cực Lạc của Phật A Di Đà và phương pháp niệm Phật vãng sinh.',
       (SELECT id FROM book_categories WHERE slug = 'phat-gia'),
       'vi', 'single', 'published', true
WHERE NOT EXISTS (SELECT 1 FROM books WHERE slug = 'kinh-a-di-qua');

INSERT INTO book_chapters (book_id, slug, title, content, sort_order)
SELECT (SELECT id FROM books WHERE slug = 'kinh-a-di-qua'),
       'toan-bo', 'Toàn Bộ Kinh',
       E'Như vậy tôi nghe. Một thời Phật ở tại vườn Kỳ Thọ Cấp Cô Độc, nước Xá Vệ, cùng với đoàn đại Tỳ-kheo một ngàn hai trăm năm mươi người.\n\nĐều là các vị A-la-hán, đại đệ tử của Phật, chúng đều là những bậc đã đoạn trừ các lậu hoặc, không còn phiền não, tâm tự tại giải thoát, đã tu tập đến bờ kia của giải thoát.\n\nLại có các vị Bồ-tát Ma-ha-tát: Văn Thù Sư Lợi Pháp Vương Tử, A Dũ Đà Bồ-tát, Càn Đà Hạt Đề Bồ-tát, Thường Tinh Tấn Bồ-tát, cùng các vị Bồ-tát khác như vậy, cùng một lúc đến dự hội.\n\n---\n\nLúc đó Phật bảo trưởng lão Xá Lợi Phất: "Từ đây qua mười vạn ức cõi Phật về phương Tây, có thế giới tên là Cực Lạc. Trong cõi đó có Phật hiệu là A Di Đà, nay đang thuyết pháp.\n\nXá Lợi Phất! Vì sao cõi đó tên là Cực Lạc? Vì các chúng sinh ở cõi đó không có các nỗi khổ, chỉ thọ các niềm vui, nên tên là Cực Lạc."\n\n---\n\nLại nữa Xá Lợi Phất! Cõi Cực Lạc có bảy tầng lan can, bảy tầng lưới, bảy hàng cây, đều bằng bốn loại báu, bao bọc xung quanh. Vì thế cõi đó gọi là Cực Lạc.\n\n---\n\nXá Lợi Phất! Nếu có thiện nam tử, thiện nữ nhân, nghe nói đến Phật A Di Đà, liền chấp trì danh hiệu của Ngài, hoặc một ngày, hoặc hai ngày, hoặc ba ngày, hoặc bốn ngày, hoặc năm ngày, hoặc sáu ngày, hoặc bảy ngày, nhất tâm bất loạn.\n\nNgười đó lúc sắp mạng chung, Phật A Di Đà cùng các thánh chúng sẽ hiện ra trước mặt. Người đó lúc mạng chung, tâm không điên đảo, liền được vãng sinh về cõi Cực Lạc của Phật A Di Đà.\n\n---\n\nXá Lợi Phất! Ta thấy được lợi ích đó nên nói lời như vậy. Nếu có chúng sinh nghe lời nói đó, nên phát nguyện nguyện được sinh về cõi Cực Lạc.\n\nNguyện công đức vô lượng. Nam Mô A Di Đà Phật.',
       1
WHERE NOT EXISTS (SELECT 1 FROM book_chapters bc JOIN books b ON b.id = bc.book_id WHERE b.slug = 'kinh-a-di-qua' AND bc.slug = 'toan-bo');

INSERT INTO books (slug, title, author, description, category_id, language, book_type, status, is_featured)
SELECT 'dao-duc-kinh', 'Đạo Đức Kinh', 'Lão Tử',
       'Đạo Đức Kinh (道德經) là tác phẩm kinh điển của Đạo gia, do Lão Tử soạn. Sách gồm 81 chương, chứa đựng triết lý về Đạo — nguồn gốc của vũ trụ, và Đức — cách ứng xử hợp Đạo. Ảnh hưởng sâu rộng đến triết học, văn hóa Đông Á.',
       (SELECT id FROM book_categories WHERE slug = 'dao-gia'),
       'vi', 'multi', 'published', true
WHERE NOT EXISTS (SELECT 1 FROM books WHERE slug = 'dao-duc-kinh');

INSERT INTO book_chapters (book_id, slug, title, content, sort_order)
SELECT (SELECT id FROM books WHERE slug = 'dao-duc-kinh'),
       'chuong-1', 'Chương 1 — Huyền Đức',
       E'Đạo khả đạo, phi thường đạo.\nDanh khả danh, phi thường danh.\nVô, danh thiên địa chi thủy.\nHữu, danh vạn vật chi mẫu.\n\nCho nên:\n- Thường với "vô", muốn quan cái diệu của Đạo.\n- Thường với "hữu", muốn quan cái kiệt của Đạo.\n\nHai cái này đồng mà xuất, dị danh mà đồng vị. Vì đồng nên gọi là Huyền.\n\nHuyền chi hựu huyền, chúng diệu chi môn.\n\n---\n\nDịch nghĩa:\nĐạo có thể nói ra được, không phải là đạo thường hằng.\nTên có thể gọi được, không phải là tên thường hằng.\n\nKhông, là tên của trời đất khi khởi đầu.\nCó, là tên của vạn vật khi sinh ra.\n\nVậy nên thường ở chỗ "không" để xem cái diệu của Đạo.\nThường ở chỗ "có" để xem cái biến của Đạo.\n\nHai cái đó (có và không) cùng xuất từ một nguồn, nhưng tên gọi khác nhau. Cùng gọi là "Huyền" (sâu xa mầu nhiệm).\n\nHuyền lại càng huyền, là cửa của mọi điều mầu nhiệm.',
       1
WHERE NOT EXISTS (SELECT 1 FROM book_chapters bc JOIN books b ON b.id = bc.book_id WHERE b.slug = 'dao-duc-kinh' AND bc.slug = 'chuong-1');

INSERT INTO book_chapters (book_id, slug, title, content, sort_order)
SELECT (SELECT id FROM books WHERE slug = 'dao-duc-kinh'),
       'chuong-2', 'Chương 2 — Vô Vi',
       E'Thiên hạ giai tri mỹ chi vi mỹ, t ác ác dĩ.\nGiai tri thiện chi vi thiện, tư bất thiện dĩ.\n\nCố hữu vô tương sinh, nan dị tương thành, trường đoản tương hình, cao hạ tương khuynh, âm thanh tương hoà, tiền hậu tương tùy.\n\nCho nên thánh nhân xử vô vi chi sự, hành bất ngôn chi giáo. Vạn vật tác nhi bất từ, sinh nhi bất hữu, vi nhi bất trì, công thành nhi弗 cư.\n\nPhu duy弗 cư, thị dĩ bất khứ.\n\n---\n\nDịch nghĩa:\nThiên hạ đều biết đẹp là đẹp, thì đã có xấu rồi.\nĐều biết thiện là thiện, thì đã có bất thiện rồi.\n\nVậy nên có và không sinh ra nhau,\nKhó và dễ thành tựu nhau,\nDài và ngắn hình dung nhau,\nCao và thấp khuynh tà nhau,\nÂm và thanh hòa nhau,\nTrước và sau theo nhau.\n\nCho nên thánh nhân làm việc vô vi, hành đạo không lời. Vạn vật sinh sôi mà không can thiệp, sinh ra mà không chiếm hữu, làm mà không dựa vào, công thành mà không tự cho là công.\n\nChỉ vì không tự cho là công, nên công đức không mất.',
       2
WHERE NOT EXISTS (SELECT 1 FROM book_chapters bc JOIN books b ON b.id = bc.book_id WHERE b.slug = 'dao-duc-kinh' AND bc.slug = 'chuong-2');

INSERT INTO book_chapters (book_id, slug, title, content, sort_order)
SELECT (SELECT id FROM books WHERE slug = 'dao-duc-kinh'),
       'chuong-8', 'Chương 8 — Thượng Thiện',
       E'Thượng thiện nhược thủy.\nThiện lợi vạn vật nhi bất tranh,\nxử chúng nhân chi sở ác,\ncố kỷ ư đạo.\n\nCư thiện địa, tâm thiện uyên,\nhữ thiện nhân, ngôn thiện tín,\nchính thiện trị, sự thiện năng,\nđộng thiện thời.\n\nPhu duy bất tranh, cố vô vưu.\n\n---\n\nDịch nghĩa:\nBậc thiện cao nhất thì như nước.\nNước làm lợi cho vạn vật mà không tranh,\nỞ chỗ mọi người ghét,\nCho nên gần với Đạo.\n\nỞ chọn đất thấp, tâm chọn chỗ sâu tĩnh,\nGiao thiệp chọn điều nhân, nói năng chọn chữ tín,\nChính chọn sự trị, làm việc chọn năng lực,\nHành động chọn thì.\n\nChỉ vì không tranh, cho nên không lầm lỗi.',
       8
WHERE NOT EXISTS (SELECT 1 FROM book_chapters bc JOIN books b ON b.id = bc.book_id WHERE b.slug = 'dao-duc-kinh' AND bc.slug = 'chuong-8');

INSERT INTO books (slug, title, author, description, category_id, language, book_type, status, is_featured)
SELECT 'kinh-tam-da-hai', 'Kinh Tam Đại Hải', 'Nguyên Thuyết (Trích)',
       'Kinh Tam Đại Hái (大方广佛华严经) là một trong những kinh quan trọng bậc nhất của Phật giáo Đại thừa, thuộc hệ kinh Hoa Nghiêm. Kinh giảng về pháp giới duy tâm, sự sự vô ngại, cảnh giới của Bồ-tát Địa Tạng Vương và nguyện lực cứu độ chúng sinh.',
       (SELECT id FROM book_categories WHERE slug = 'phat-gia'),
       'vi', 'single', 'published', false
WHERE NOT EXISTS (SELECT 1 FROM books WHERE slug = 'kinh-tam-da-hai');

INSERT INTO book_chapters (book_id, slug, title, content, sort_order)
SELECT (SELECT id FROM books WHERE slug = 'kinh-tam-da-hai'),
       'phat-nguyen', 'Phát Nguyện',
       E'Nam Mô Đại Từ Đại Bi Cứu Khổ Cứu Nạn Quảng Đại Linh Cảm Địa Tạng Vương Bồ Tát.\n\nKính lạy Địa Tạng Vương Bồ Tát, bậc Đại Sĩ đã phát nguyện rộng lớn: "Địa ngục chưa trống, thề chưa thành Phật; chúng sinh độ hết, mới chứng Bồ Đề."\n\nNguyện rằng:\n- Cho tất cả chúng sinh trong mười phương, đều lìa mọi nỗi khổ.\n- Cho tất cả những ai đang đoạ địa ngục, ngạ quỷ, súc sinh, đều được siêu thoát.\n- Cho tất cả người thân quyến thuộc của con, đã khuất từ vô lượng kiếp đến nay, đều được sinh về cõi lành.\n\nNam Mô Địa Tạng Vương Bồ Tát Ma Ha Tát.\n\nNguyện công đức vô lượng. Nam Mô A Di Đà Phật.',
       1
WHERE NOT EXISTS (SELECT 1 FROM book_chapters bc JOIN books b ON b.id = bc.book_id WHERE b.slug = 'kinh-tam-da-hai' AND bc.slug = 'phat-nguyen');

INSERT INTO books (slug, title, author, description, category_id, language, book_type, status, is_featured)
SELECT 'kinh-phap-cu', 'Kinh Pháp Cú', 'Nguyên Thuyết',
       'Kinh Pháp Cú (Dhammapada) là tập hợp 423 bài kệ Phật giáo, thuộc tạng Kinh của Nam truyền Phật giáo. Mỗi bài kệ là một hạt ngọc trí tuệ, tóm tắt cốt tủy đạo Phật về nhân quả, nghiệp, từ bi, trí tuệ.',
       (SELECT id FROM book_categories WHERE slug = 'phat-gia'),
       'vi', 'multi', 'published', true
WHERE NOT EXISTS (SELECT 1 FROM books WHERE slug = 'kinh-phap-cu');

INSERT INTO book_chapters (book_id, slug, title, content, sort_order)
SELECT (SELECT id FROM books WHERE slug = 'kinh-phap-cu'),
       'chuong-1', 'Yamaka Vagga — Phần Kép',
       E'1. Tâm là nguồn của mọi pháp, tâm là chủ, tâm tạo tác.\nNếu nói hay làm với tâm ô nhiễm,\nKhổ đau theo như xe kéo con vật.\n\n2. Tâm là nguồn của mọi pháp, tâm là chủ, tâm tạo tác.\nNếu nói hay làm với tâm thanh tịnh,\nHạnh phúc theo như bóng theo hình.\n\n3. "Hắn đã mắng tôi, đánh tôi, thắng tôi, cướp tôi."\nAi ôm giữ tâm ấy, hận thận không dứt.\n\n4. "Hắn đã mắng tôi, đánh tôi, thắng tôi, cướp tôi."\nAi không ôm giữ tâm ấy, hận thận tự hết.\n\n5. Không phải bởi hận thù mà hận thù dứt được.\nChỉ bởi không hận thù mà hận thù dứt. Đây là luật vĩnh cửu.\n\n---\n\nTâm ta là chủ của mọi sự. Tâm thanh tịnh thì sự thanh tịnh, tâm ô nhiễm thì sự ô nhiễm. Hạnh phúc hay khổ đau, không ai ban, không ai lấy đi, chỉ do tâm ta tạo.\n\nNguyện công đức vô lượng. Nam Mô A Di Đà Phật.',
       1
WHERE NOT EXISTS (SELECT 1 FROM book_chapters bc JOIN books b ON b.id = bc.book_id WHERE b.slug = 'kinh-phap-cu' AND bc.slug = 'chuong-1');

-- 10. Comments
COMMENT ON TABLE book_categories IS '5 thư viện chính: Phật Gia, Đạo Gia, Kinh Văn, Sách Quý, Quan Trọng';
COMMENT ON TABLE books IS 'Sách điện tử trong thư viện Kinh Sách';
COMMENT ON COLUMN books.book_type IS 'single (một bài) | multi (nhiều chương)';
COMMENT ON COLUMN books.status IS 'draft | pending_review | published | archived';
COMMENT ON TABLE book_chapters IS 'Chương mục của sách (chỉ áp dụng cho book_type=multi)';
COMMENT ON TABLE book_reviews IS 'Cảm ngộ của thành viên — phải có tối thiểu 100 chữ và qua xét duyệt mới hiển thị';
COMMENT ON COLUMN book_reviews.status IS 'pending | approved | rejected';
COMMENT ON TABLE book_donations IS 'Kính (Donate K) cho sách — sẽ link sang Quỹ Từ Bi sau khi có hệ thống tiền tệ';
COMMENT ON TABLE book_flowers IS 'Tặng hoa — 1 user chỉ tặng 1 lần/sách (unique index)';
