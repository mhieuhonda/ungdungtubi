/**
 * Ứng Dụng Từ Bi — Sound Effects Module (v0.9.20 — Giai đoạn 25)
 *
 * Sử dụng Web Audio API để tạo sound effects dynamically — không cần file audio.
 * Ưu điểm:
 *   - Không load thêm bytes (no HTTP requests)
 *   - Hoạt động offline
 *   - Lazy init: chỉ tạo AudioContext khi user tương tác lần đầu
 *   - Respect sound toggle trong settings
 *
 * Sounds:
 *   - playSendSound():   pop ngắn khi user gửi tin nhắn (440Hz → 880Hz, 80ms)
 *   - playReceiveSound(): chime nhẹ khi nhận tin nhắn (660Hz, 120ms)
 *   - playConnectSound(): bell khi WS kết nối thành công (523Hz → 659Hz, 200ms)
 *   - playErrorSound():   buzzer ngắn khi có lỗi (200Hz, 150ms)
 */

const TubiSound = (function () {
    let audioCtx = null;
    let enabled = true;

    // Load preference from localStorage (default: enabled)
    try {
        const pref = localStorage.getItem('tubi_sound');
        if (pref !== null) {
            enabled = pref === 'true';
        }
    } catch (_) {}

    function getCtx() {
        if (!enabled) return null;
        if (audioCtx) return audioCtx;
        try {
            const Ctx = window.AudioContext || window.webkitAudioContext;
            if (!Ctx) return null;
            audioCtx = new Ctx();
            // Resume if suspended (browser autoplay policy)
            if (audioCtx.state === 'suspended') {
                audioCtx.resume().catch(() => {});
            }
            return audioCtx;
        } catch (_) {
            return null;
        }
    }

    /**
     * Phát một tone đơn giản với envelope ADSR rút gọn.
     * @param {number} freq - Tần số (Hz)
     * @param {number} duration - Độ dài (ms)
     * @param {number} volume - 0..1
     * @param {OscillatorType} type - sine | triangle | square | sawtooth
     * @param {number} freqEnd - Tần số kết thúc (cho glide), mặc định = freq
     */
    function playTone(freq, duration, volume, type, freqEnd) {
        const ctx = getCtx();
        if (!ctx) return;

        try {
            const osc = ctx.createOscillator();
            const gain = ctx.createGain();

            osc.type = type || 'sine';
            osc.frequency.setValueAtTime(freq, ctx.currentTime);
            if (freqEnd && freqEnd !== freq) {
                osc.frequency.exponentialRampToValueAtTime(
                    freqEnd,
                    ctx.currentTime + duration / 1000
                );
            }

            // Envelope: attack 10ms → sustain → release 30ms
            const t0 = ctx.currentTime;
            const dur = duration / 1000;
            gain.gain.setValueAtTime(0, t0);
            gain.gain.linearRampToValueAtTime(volume, t0 + 0.01);
            gain.gain.setValueAtTime(volume, t0 + dur - 0.03);
            gain.gain.linearRampToValueAtTime(0, t0 + dur);

            osc.connect(gain);
            gain.connect(ctx.destination);

            osc.start(t0);
            osc.stop(t0 + dur);
        } catch (_) {}
    }

    return {
        /** Bật/tắt sound effects. */
        setEnabled(v) {
            enabled = !!v;
            try {
                localStorage.setItem('tubi_sound', enabled ? 'true' : 'false');
            } catch (_) {}
            if (enabled) getCtx(); // pre-init
        },
        isEnabled() {
            return enabled;
        },
        /** Toggle on/off, trả về state mới. */
        toggle() {
            this.setEnabled(!enabled);
            return enabled;
        },

        /** Pop ngắn khi user gửi tin nhắn — cảm giác "ting" nhẹ. */
        playSend() {
            // 880Hz → 1320Hz, 70ms, triangle — bright pop
            playTone(880, 70, 0.08, 'triangle', 1320);
        },

        /** Chime nhẹ khi nhận tin nhắn — không gây phiền. */
        playReceive() {
            // 660Hz, 100ms, sine — soft bell
            playTone(660, 100, 0.06, 'sine');
        },

        /** Bell 2 nốt khi WS kết nối thành công. */
        playConnect() {
            playTone(523, 120, 0.05, 'sine'); // C5
            setTimeout(() => playTone(659, 150, 0.05, 'sine'), 110); // E5
        },

        /** Buzzer ngắn khi có lỗi. */
        playError() {
            playTone(200, 150, 0.05, 'square');
        },

        /**
         * Init audio context trên first user interaction.
         * Browser autoplay policy yêu cầu user gesture trước khi play audio.
         */
        initOnInteraction() {
            const init = () => {
                getCtx();
                document.removeEventListener('click', init);
                document.removeEventListener('keydown', init);
                document.removeEventListener('touchstart', init);
            };
            document.addEventListener('click', init, { once: true, passive: true });
            document.addEventListener('keydown', init, { once: true, passive: true });
            document.addEventListener('touchstart', init, { once: true, passive: true });
        },
    };
})();

window.TubiSound = TubiSound;

// Auto-init on first interaction
if (document.readyState !== 'loading') {
    TubiSound.initOnInteraction();
} else {
    document.addEventListener('DOMContentLoaded', () => TubiSound.initOnInteraction());
}
