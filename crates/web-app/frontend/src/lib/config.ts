interface RuntimeConfig {
    TELEGRAM_BOT_NAME?: string;
    // Add other config values here as needed
}

// This will be injected by the Rust application
declare global {
    interface Window {
        RUNTIME_CONFIG?: RuntimeConfig;
    }
}

export function getConfig(): RuntimeConfig {
    return window.RUNTIME_CONFIG || {};
}

export function getTelegramBotName(): string | undefined {
    return getConfig().TELEGRAM_BOT_NAME;
}

// Add other config getters as needed 