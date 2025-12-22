// ============================================================
// Email Verification Module (UX-003)
// ============================================================

const VERIFICATION_KEY = 'docsign_email_verification';
const LOCKOUT_KEY = 'docsign_verification_lockout';
const MAX_ATTEMPTS = 3;
const LOCKOUT_DURATION_MS = 15 * 60 * 1000; // 15 minutes

/**
 * Check if email verification is already complete
 */
export function isEmailVerified() {
    const verificationData = localStorage.getItem(VERIFICATION_KEY);
    if (!verificationData) return false;

    try {
        const data = JSON.parse(verificationData);
        const key = `${window.sessionParams.sessionId}_${window.sessionParams.recipientId}`;
        return data[key] === true;
    } catch (e) {
        return false;
    }
}

/**
 * Mark email as verified
 */
export function markEmailVerified() {
    const verificationData = JSON.parse(localStorage.getItem(VERIFICATION_KEY) || '{}');
    const key = `${window.sessionParams.sessionId}_${window.sessionParams.recipientId}`;
    verificationData[key] = true;
    localStorage.setItem(VERIFICATION_KEY, JSON.stringify(verificationData));
}

/**
 * Get verification lockout state
 */
export function getVerificationLockout() {
    const lockoutData = localStorage.getItem(LOCKOUT_KEY);
    if (!lockoutData) return null;

    try {
        const data = JSON.parse(lockoutData);
        const key = `${window.sessionParams.sessionId}_${window.sessionParams.recipientId}`;
        const lockout = data[key];

        if (!lockout) return null;

        // Check if lockout has expired
        if (Date.now() > lockout.expiresAt) {
            delete data[key];
            localStorage.setItem(LOCKOUT_KEY, JSON.stringify(data));
            return null;
        }

        return lockout;
    } catch (e) {
        return null;
    }
}

/**
 * Set verification lockout
 */
export function setVerificationLockout() {
    const lockoutData = JSON.parse(localStorage.getItem(LOCKOUT_KEY) || '{}');
    const key = `${window.sessionParams.sessionId}_${window.sessionParams.recipientId}`;
    lockoutData[key] = {
        attempts: MAX_ATTEMPTS,
        expiresAt: Date.now() + LOCKOUT_DURATION_MS,
        lockedAt: Date.now()
    };
    localStorage.setItem(LOCKOUT_KEY, JSON.stringify(lockoutData));
}

/**
 * Get verification attempts
 */
export function getVerificationAttempts() {
    const lockoutData = localStorage.getItem(LOCKOUT_KEY);
    if (!lockoutData) return 0;

    try {
        const data = JSON.parse(lockoutData);
        const key = `${window.sessionParams.sessionId}_${window.sessionParams.recipientId}`;
        return data[key]?.attempts || 0;
    } catch (e) {
        return 0;
    }
}

/**
 * Increment verification attempts
 */
export function incrementVerificationAttempts() {
    const lockoutData = JSON.parse(localStorage.getItem(LOCKOUT_KEY) || '{}');
    const key = `${window.sessionParams.sessionId}_${window.sessionParams.recipientId}`;

    if (!lockoutData[key]) {
        lockoutData[key] = { attempts: 0 };
    }

    lockoutData[key].attempts = (lockoutData[key].attempts || 0) + 1;

    if (lockoutData[key].attempts >= MAX_ATTEMPTS) {
        lockoutData[key].expiresAt = Date.now() + LOCKOUT_DURATION_MS;
        lockoutData[key].lockedAt = Date.now();
    }

    localStorage.setItem(LOCKOUT_KEY, JSON.stringify(lockoutData));

    return lockoutData[key].attempts;
}

/**
 * Show email verification screen
 */
export function showEmailVerification(recipientEmail, showConsentLanding) {
    console.log('[Verification] Showing verification for:', recipientEmail);

    const verificationScreen = document.getElementById('verification-screen');
    const maskedEmailEl = document.getElementById('masked-email');
    const suffixInput = document.getElementById('email-suffix-input');
    const verifyButton = document.getElementById('verify-button');
    const errorDiv = document.getElementById('verification-error');
    const lockoutDiv = document.getElementById('lockout-message');
    const errorMessage = document.getElementById('error-message');
    const attemptsRemaining = document.getElementById('attempts-remaining');
    const lockoutTimer = document.getElementById('lockout-timer');
    const loadingIndicator = document.getElementById('loading-indicator');

    // Hide loading, show verification screen
    loadingIndicator.classList.add('hidden');
    verificationScreen.classList.remove('hidden');

    // Check for active lockout
    const lockout = getVerificationLockout();
    if (lockout) {
        verifyButton.disabled = true;
        suffixInput.disabled = true;
        lockoutDiv.classList.remove('hidden');
        errorDiv.classList.add('hidden');

        // Update lockout timer
        function updateTimer() {
            const remaining = Math.max(0, Math.ceil((lockout.expiresAt - Date.now()) / 1000));
            const minutes = Math.floor(remaining / 60);
            const seconds = remaining % 60;
            lockoutTimer.textContent = `${minutes}:${seconds.toString().padStart(2, '0')}`;

            if (remaining <= 0) {
                window.location.reload();
            }
        }

        updateTimer();
        setInterval(updateTimer, 1000);

        return;
    }

    // Mask the email using WASM function if available
    let maskedEmail = recipientEmail;
    if (window.wasmModule && window.wasmModule.mask_email) {
        try {
            maskedEmail = window.wasmModule.mask_email(recipientEmail);
        } catch (e) {
            // Fallback to simple masking
            const atPos = recipientEmail.indexOf('@');
            if (atPos > 0) {
                maskedEmail = recipientEmail[0] + '***@' + recipientEmail.substring(atPos + 1);
            }
        }
    } else {
        // JavaScript fallback
        const atPos = recipientEmail.indexOf('@');
        if (atPos > 0) {
            maskedEmail = recipientEmail[0] + '***@' + recipientEmail.substring(atPos + 1);
        }
    }

    maskedEmailEl.textContent = maskedEmail;

    // Verify button handler
    verifyButton.onclick = () => {
        const suffix = suffixInput.value.trim().toLowerCase();

        if (!suffix || suffix.length < 1) {
            errorMessage.textContent = 'Please enter at least 1 character.';
            errorDiv.classList.remove('hidden');
            return;
        }

        // Verify using WASM function if available
        let isValid = false;
        if (window.wasmModule && window.wasmModule.verify_email_suffix) {
            try {
                isValid = window.wasmModule.verify_email_suffix(recipientEmail, suffix);
            } catch (e) {
                // Fallback to simple verification
                isValid = recipientEmail.toLowerCase().endsWith(suffix);
            }
        } else {
            // JavaScript fallback
            isValid = recipientEmail.toLowerCase().endsWith(suffix);
        }

        if (isValid) {
            // Success!
            markEmailVerified();
            console.log('[Verification] Email verified successfully');

            // Hide verification screen, show consent landing
            verificationScreen.classList.add('hidden');
            showConsentLanding();
        } else {
            // Failed attempt
            const attempts = incrementVerificationAttempts();
            const remaining = MAX_ATTEMPTS - attempts;

            if (remaining <= 0) {
                // Lockout
                setVerificationLockout();
                window.location.reload();
            } else {
                // Show error
                errorMessage.textContent = 'Incorrect input. Please try again.';
                attemptsRemaining.textContent = `${remaining} attempt${remaining !== 1 ? 's' : ''} remaining`;
                errorDiv.classList.remove('hidden');
                suffixInput.value = '';
                suffixInput.focus();
            }
        }
    };

    // Allow Enter key to verify
    suffixInput.onkeypress = (e) => {
        if (e.key === 'Enter') {
            verifyButton.click();
        }
    };

    // Focus input
    suffixInput.focus();
}
