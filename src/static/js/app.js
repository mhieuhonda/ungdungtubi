/**
 * Ứng Dụng Từ Bi - Client-side JavaScript
 * Domain: tubi.louis.vangioitutien.com
 * 
 * HTMX + Alpine.js helpers
 */

// HTMX configuration
document.addEventListener('htmx:configRequest', function(event) {
    // Add CSRF token if available
    const token = document.querySelector('meta[name="csrf-token"]');
    if (token) {
        event.detail.headers['X-CSRF-Token'] = token.content;
    }
});

// Prayer counter with localStorage persistence
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
    }
};

// Session heartbeat (keep session alive)
function sessionHeartbeat() {
    setInterval(() => {
        fetch('/api/heartbeat', { method: 'POST', credentials: 'same-origin' })
            .catch(() => {}); // Silently fail
    }, 5 * 60 * 1000); // Every 5 minutes
}

// Initialize
document.addEventListener('DOMContentLoaded', function() {
    // Start session heartbeat if logged in
    if (document.cookie.includes('session_id')) {
        sessionHeartbeat();
    }
    
    // Service Worker registration (for background prayer counting)
    if ('serviceWorker' in navigator) {
        // Will be implemented in Phase 5
    }
});

// Alpine.js global store
document.addEventListener('alpine:init', () => {
    Alpine.store('tubi', {
        // Current user info
        user: null,
        
        // Prayer state
        prayer: {
            count: PrayerCounter.getCount(),
            isRunning: false,
        },
        
        // Currency
        a: 0,
        k: 0,
        
        // Methods
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

// Utility: Format number with Vietnamese locale
function formatNumber(num) {
    return new Intl.NumberFormat('vi-VN').format(num);
}

// Utility: Time ago in Vietnamese
function timeAgo(date) {
    const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
    
    if (seconds < 60) return 'vừa xong';
    if (seconds < 3600) return `${Math.floor(seconds / 60)} phút trước`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)} giờ trước`;
    if (seconds < 604800) return `${Math.floor(seconds / 86400)} ngày trước`;
    
    return new Intl.DateTimeFormat('vi-VN').format(date);
}

// ====================================================================
// Live Chat Alpine.js component — v0.9.2 Giai đoạn 7
// WebSocket real-time chat trong nhóm cộng đồng
//
// Cách dùng (trong template):
//   <div x-data="liveChat({ slug: '...', isMember: true, initialMessages: [...] })" x-init="init()">
// ====================================================================

function liveChat(opts) {
    return {
        // --- State ---
        slug: opts.slug || '',
        isMember: opts.isMember === true,
        isLoggedIn: opts.isLoggedIn === true,
        messages: Array.isArray(opts.initialMessages) ? opts.initialMessages.slice() : [],
        draft: '',
        connected: false,
        error: '',
        socket: null,
        reconnectAttempts: 0,
        maxReconnectAttempts: 5,
        reconnectTimer: null,
        onlineCount: 0,

        // Computed label for online status
        get onlineLabel() {
            if (this.onlineCount > 0) {
                return `${this.onlineCount} người online`;
            }
            return 'đã kết nối';
        },

        // --- Lifecycle ---
        init() {
            // Chỉ kết nối WebSocket nếu đã đăng nhập + là member active
            if (!this.isLoggedIn) {
                this.error = 'Cần đăng nhập để xem chat';
                return;
            }
            if (!this.isMember) {
                this.error = 'Tham gia nhóm để chat';
                return;
            }
            // Auto-scroll xuống cuối sau khi render messages ban đầu
            this.$nextTick(() => this.scrollToBottom());
            this.connect();
        },

        // --- WebSocket ---
        connect() {
            if (this.socket) {
                try { this.socket.close(); } catch (_) {}
                this.socket = null;
            }

            // Xây URL WebSocket:
            //   production: wss://<host>/ws/cong-dong/nhom/<slug>
            //   dev:        ws://<host>/ws/cong-dong/nhom/<slug>
            const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const host = window.location.host;
            const url = `${proto}//${host}/ws/cong-dong/nhom/${encodeURIComponent(this.slug)}`;

            try {
                this.socket = new WebSocket(url);
            } catch (e) {
                this.error = 'Trình duyệt không hỗ trợ WebSocket';
                return;
            }

            this.socket.onopen = () => {
                this.connected = true;
                this.error = '';
                this.reconnectAttempts = 0;
            };

            this.socket.onmessage = (event) => {
                this.handleIncoming(event.data);
            };

            this.socket.onclose = (event) => {
                this.connected = false;
                // 1000 = normal close, 1008 = policy violation (auth/permission fail)
                if (event.code === 1008) {
                    this.error = event.reason || 'Không có quyền chat';
                    return; // không reconnect
                }
                // Reconnect với backoff
                this.scheduleReconnect();
            };

            this.socket.onerror = () => {
                this.connected = false;
                this.error = 'Lỗi kết nối WebSocket';
            };
        },

        scheduleReconnect() {
            if (this.reconnectAttempts >= this.maxReconnectAttempts) {
                this.error = `Không thể kết nối sau ${this.maxReconnectAttempts} lần thử. Tải lại trang.`;
                return;
            }
            this.reconnectAttempts += 1;
            const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
            this.error = `Mất kết nối — thử lại sau ${Math.round(delay/1000)}s…`;
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = setTimeout(() => this.connect(), delay);
        },

        // --- Incoming message handler ---
        handleIncoming(raw) {
            let data;
            try {
                data = JSON.parse(raw);
            } catch (_) {
                return; // bỏ qua payload không phải JSON
            }

            // Error payload từ server
            if (data.type === 'error' && typeof data.message === 'string') {
                this.error = data.message;
                setTimeout(() => { this.error = ''; }, 3000);
                return;
            }

            // Chat message payload
            if (data.id && data.body && data.author_display_name) {
                // Tránh duplicate (trường hợp server echo về tin mình vừa gửi)
                if (this.messages.some(m => m.id === data.id)) return;
                this.messages.push(data);
                this.$nextTick(() => this.scrollToBottom());
            }
        },

        // --- Send message ---
        send() {
            const body = this.draft.trim();
            if (!body) return;
            if (!this.connected || !this.socket) {
                this.error = 'Chưa kết nối — vui lòng đợi';
                return;
            }
            if (body.length > 500) {
                this.error = 'Tin nhắn quá dài (tối đa 500 ký tự)';
                return;
            }

            try {
                this.socket.send(body);
                this.draft = '';
                this.error = '';
            } catch (e) {
                this.error = 'Không gửi được tin nhắn';
            }
        },

        // --- Helpers ---
        scrollToBottom() {
            const el = this.$refs.messages;
            if (el) {
                el.scrollTop = el.scrollHeight;
            }
        },

        formatTime(isoStr) {
            try {
                const dt = new Date(isoStr);
                return new Intl.DateTimeFormat('vi-VN', {
                    hour: '2-digit',
                    minute: '2-digit',
                    day: '2-digit',
                    month: '2-digit',
                }).format(dt);
            } catch (_) {
                return '';
            }
        },
    };
}

// Expose globally for Alpine.js x-data
window.liveChat = liveChat;
