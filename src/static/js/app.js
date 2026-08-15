/**
 * Ứng Dụng Từ Bi - Client-side JavaScript (main entry)
 * Domain: tubi.louis.vangioitutien.com
 *
 * v0.9.20 — Giai đoạn 25: Refactor
 *   - Chat components (liveChat, globalChat, dmChat, chatBubble, notificationBadge)
 *     chuyển sang /static/js/chat.js
 *   - Sound effects chuyển sang /static/js/sound.js
 *   - File này giữ: HTMX config, PrayerCounter, session heartbeat, theme helpers,
 *     utility functions (formatNumber, timeAgo)
 *
 * Load order trong layout.html:
 *   1. /static/js/sound.js  (sound effects module)
 *   2. /static/js/chat.js   (chat Alpine.js components)
 *   3. /static/js/app.js    (this file — main init)
 */

// HTMX configuration — add CSRF token if available
document.addEventListener('htmx:configRequest', function (event) {
    const token = document.querySelector('meta[name="csrf-token"]');
    if (token) {
        event.detail.headers['X-CSRF-Token'] = token.content;
    }
});

// ====================================================================
// Prayer counter with localStorage persistence
// ====================================================================

const PrayerCounter = {
    key: 'tubi_prayer_count',

    getCount() {
        return parseInt(localStorage.getItem(this.key) || '0', 10);
    },

    setCount(count) {
        localStorage.setItem(this.key, count.toString());
    },

    increment() {
        const count = this.getCount() + 1;
        this.setCount(count);
        return count;
    },
};

// ====================================================================
// Session heartbeat (keep session alive)
// v0.9.20 FIX: trước đây check `document.cookie.includes('session_id')` nhưng
// cookie session_id là HttpOnly → document.cookie KHÔNG đọc được → heartbeat
// không bao giờ chạy → session hết hạn khi user đang active → WS auth fail →
// "không gửi được tin nhắn".
// Fix: check `<body data-logged-in="true">` thay vì cookie.
// ====================================================================

function sessionHeartbeat() {
    setInterval(() => {
        fetch('/api/heartbeat', { method: 'POST', credentials: 'same-origin' }).catch(() => {});
    }, 5 * 60 * 1000); // Every 5 minutes
}

// ====================================================================
// Sound toggle helper — expose globally cho templates
// ====================================================================

window.toggleSound = function () {
    if (!window.TubiSound) return false;
    const enabled = TubiSound.toggle();
    // Visual feedback
    const btn = document.querySelector('[data-sound-toggle]');
    if (btn) {
        btn.textContent = enabled ? '🔊' : '🔇';
        btn.title = enabled ? 'Tắt âm thanh' : 'Bật âm thanh';
    }
    return enabled;
};

// ====================================================================
// Initialize on DOM ready
// ====================================================================

document.addEventListener('DOMContentLoaded', function () {
    // v0.9.20: Start session heartbeat nếu user đã đăng nhập
    // (check <body data-logged-in="true"> — layout.html set attribute này)
    const bodyLoggedIn = document.body && document.body.dataset.loggedIn === 'true';
    if (bodyLoggedIn) {
        sessionHeartbeat();
    }

    // Service Worker registration (for background prayer counting) — Phase 5
    if ('serviceWorker' in navigator) {
        // Will be implemented in Phase 5
    }
});

// ====================================================================
// Alpine.js global store
// ====================================================================

document.addEventListener('alpine:init', () => {
    Alpine.store('tubi', {
        user: null,
        prayer: {
            count: PrayerCounter.getCount(),
            isRunning: false,
        },
        a: 0,
        k: 0,

        incrementPrayer() {
            this.prayer.count = PrayerCounter.increment();
            this.a++;
            if (this.a >= 1000) {
                this.k++;
                this.a -= 1000;
            }
        },
    });
});

// ====================================================================
// Utility functions
// ====================================================================

function formatNumber(num) {
    return new Intl.NumberFormat('vi-VN').format(num);
}

function timeAgo(date) {
    const seconds = Math.floor((Date.now() - date.getTime()) / 1000);

    if (seconds < 60) return 'vừa xong';
    if (seconds < 3600) return `${Math.floor(seconds / 60)} phút trước`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)} giờ trước`;
    if (seconds < 604800) return `${Math.floor(seconds / 86400)} ngày trước`;

    return new Intl.DateTimeFormat('vi-VN').format(date);
}

window.formatNumber = formatNumber;
window.timeAgo = timeAgo;
window.PrayerCounter = PrayerCounter;
