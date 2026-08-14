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
                // v0.9.3: server expects plain text, not JSON
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

// ====================================================================
// Global Chat (Chat Chung) Alpine.js component — v0.9.3
// Platform-wide chat accessible from any page via draggable bubble
//
// Cách dùng (trong layout):
//   <div x-data="globalChat()" x-init="init()">
// ====================================================================

function globalChat() {
    return {
        // --- State ---
        messages: [],
        draft: '',
        connected: false,
        error: '',
        socket: null,
        reconnectAttempts: 0,
        maxReconnectAttempts: 5,
        reconnectTimer: null,
        isOpen: false,
        unreadCount: 0,
        initialized: false,

        // --- Lifecycle ---
        // [v0.9.5 fix] Bỏ check `document.cookie.includes('session_id')` vì
        // cookie `session_id` là HttpOnly → `document.cookie` không đọc được.
        // Layout đã chỉ render global chat khi user đăng nhập (`{% if let Some(_u) = user %}`),
        // nên không cần check lại ở client. Server sẽ trả 401 nếu chưa đăng nhập.
        init() {
            this.initialized = true;
            // Tải history ban đầu
            this.loadHistory();
            this.connect();
        },

        // --- Load history ---
        async loadHistory() {
            try {
                const resp = await fetch('/api/chat-chung/history?limit=50', { credentials: 'same-origin' });
                if (resp.ok) {
                    const msgs = await resp.json();
                    // Server trả newest first, reverse để oldest first
                    this.messages = msgs.reverse();
                }
            } catch (_) {}
        },

        // --- WebSocket ---
        connect() {
            if (this.socket) {
                try { this.socket.close(); } catch (_) {}
                this.socket = null;
            }

            const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const host = window.location.host;
            const url = `${proto}//${host}/ws/chat-chung`;

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
                if (event.code === 1008) {
                    this.error = event.reason || 'Không có quyền chat';
                    return;
                }
                this.scheduleReconnect();
            };

            this.socket.onerror = () => {
                this.connected = false;
                this.error = 'Lỗi kết nối';
            };
        },

        scheduleReconnect() {
            if (this.reconnectAttempts >= this.maxReconnectAttempts) {
                this.error = `Không thể kết nối sau ${this.maxReconnectAttempts} lần thử.`;
                return;
            }
            this.reconnectAttempts += 1;
            const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = setTimeout(() => this.connect(), delay);
        },

        // --- Incoming message handler ---
        handleIncoming(raw) {
            let data;
            try {
                data = JSON.parse(raw);
            } catch (_) { return; }

            if (data.type === 'error' && typeof data.message === 'string') {
                this.error = data.message;
                setTimeout(() => { this.error = ''; }, 3000);
                return;
            }

            if (data.id && data.body && data.author_display_name) {
                if (this.messages.some(m => m.id === data.id)) return;
                this.messages.push(data);
                // Tăng unread nếu popup đang đóng
                if (!this.isOpen) {
                    this.unreadCount++;
                }
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

        // --- Toggle popup ---
        toggleChat() {
            this.isOpen = !this.isOpen;
            if (this.isOpen) {
                this.unreadCount = 0;
                this.$nextTick(() => this.scrollToBottom());
            }
        },

        // --- Helpers ---
        scrollToBottom() {
            const el = this.$refs.globalMessages;
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

window.globalChat = globalChat;

// ====================================================================
// Chat Bubble (draggable) Alpine.js component — v0.9.3
// Draggable circular bubble that opens global chat popup
// ====================================================================

function chatBubble() {
    return {
        // --- Position ---
        x: 0,
        y: 0,
        startX: 0,
        startY: 0,
        offsetX: 0,
        offsetY: 0,
        dragging: false,
        moved: false,

        init() {
            // Default position: bottom-right, above mobile nav
            const isMobile = window.innerWidth < 768;
            this.x = window.innerWidth - 64;
            this.y = isMobile ? window.innerHeight - 88 : window.innerHeight - 80;
            this.offsetX = this.x;
            this.offsetY = this.y;
        },

        // --- Mouse events ---
        onMouseDown(event) {
            this.dragging = true;
            this.moved = false;
            this.startX = event.clientX - this.offsetX;
            this.startY = event.clientY - this.offsetY;
            event.preventDefault();
        },

        onMouseMove(event) {
            if (!this.dragging) return;
            this.moved = true;
            this.x = event.clientX - this.startX;
            this.y = event.clientY - this.startY;
            this.offsetX = this.x;
            this.offsetY = this.y;
        },

        onMouseUp() {
            this.dragging = false;
        },

        // --- Touch events ---
        onTouchStart(event) {
            const touch = event.touches[0];
            this.dragging = true;
            this.moved = false;
            this.startX = touch.clientX - this.offsetX;
            this.startY = touch.clientY - this.offsetY;
        },

        onTouchMove(event) {
            if (!this.dragging) return;
            this.moved = true;
            const touch = event.touches[0];
            this.x = touch.clientX - this.startX;
            this.y = touch.clientY - this.startY;
            this.offsetX = this.x;
            this.offsetY = this.y;
            event.preventDefault();
        },

        onTouchEnd() {
            this.dragging = false;
        },

        // --- Click handler (only if not dragged) ---
        onClick() {
            if (!this.moved) {
                // Dispatch custom event to toggle global chat
                this.$dispatch('toggle-global-chat');
            }
            this.moved = false;
        },

        // --- Style binding ---
        get bubbleStyle() {
            return `left: ${this.x}px; top: ${this.y}px;`;
        },
    };
}

window.chatBubble = chatBubble;

// ====================================================================
// Direct Message Chat Alpine.js component — v0.9.5 Giai đoạn 9
// 1-on-1 realtime chat via WebSocket
//
// Cách dùng (trong template):
//   <div x-data="dmChat({
//     conversationId: '...',
//     otherUserId: '...',
//     otherDisplayName: '...',
//     initialMessages: [...]
//   })" x-init="init()">
// ====================================================================

function dmChat(opts) {
    return {
        // --- State ---
        conversationId: opts.conversationId || '',
        otherUserId: opts.otherUserId || '',
        otherDisplayName: opts.otherDisplayName || '',
        messages: Array.isArray(opts.initialMessages) ? opts.initialMessages.slice() : [],
        draft: '',
        connected: false,
        error: '',
        socket: null,
        reconnectAttempts: 0,
        maxReconnectAttempts: 5,
        reconnectTimer: null,

        // --- Lifecycle ---
        init() {
            this.$nextTick(() => this.scrollToBottom());
            this.connect();
        },

        // --- WebSocket ---
        connect() {
            if (this.socket) {
                try { this.socket.close(); } catch (_) {}
                this.socket = null;
            }

            const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
            const host = window.location.host;
            const url = `${proto}//${host}/ws/ban-be/tin-nhan/${encodeURIComponent(this.conversationId)}`;

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
                if (event.code === 1008) {
                    this.error = event.reason || 'Không có quyền chat';
                    return;
                }
                this.scheduleReconnect();
            };

            this.socket.onerror = () => {
                this.connected = false;
                this.error = 'Lỗi kết nối';
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

        handleIncoming(raw) {
            let data;
            try {
                data = JSON.parse(raw);
            } catch (_) { return; }

            if (data.type === 'error' && typeof data.message === 'string') {
                this.error = data.message;
                setTimeout(() => { this.error = ''; }, 3000);
                return;
            }

            if (data.id && data.body && data.author_display_name) {
                if (this.messages.some(m => m.id === data.id)) return;
                this.messages.push(data);
                this.$nextTick(() => this.scrollToBottom());
            }
        },

        send() {
            const body = this.draft.trim();
            if (!body) return;
            if (!this.connected || !this.socket) {
                this.error = 'Chưa kết nối — vui lòng đợi';
                return;
            }
            if (body.length > 1000) {
                this.error = 'Tin nhắn quá dài (tối đa 1000 ký tự)';
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

window.dmChat = dmChat;

// ====================================================================
// Notification badge poller — v0.9.5 Giai đoạn 9
// Cứ 30s gọi /api/ban-be/thong-bao/chua-doc để cập nhật badge
// ====================================================================

function notificationBadge() {
    return {
        unreadCount: 0,
        init() {
            // Chỉ khởi tạo nếu đã đăng nhập (layout có render badge)
            this.fetchUnread();
            setInterval(() => this.fetchUnread(), 30000);
        },
        async fetchUnread() {
            try {
                const resp = await fetch('/api/ban-be/thong-bao/chua-doc', { credentials: 'same-origin' });
                if (resp.ok) {
                    const data = await resp.json();
                    this.unreadCount = data.unread_count || 0;
                }
            } catch (_) {}
        },
    };
}

window.notificationBadge = notificationBadge;
