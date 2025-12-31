/**
 * Structured Logging Utility for DocSign
 *
 * Provides environment-aware logging with namespaces, log levels,
 * and production safety. Logs are suppressed in production unless
 * explicitly enabled via URL param or localStorage.
 *
 * Usage:
 *   import { createLogger } from './logger';
 *
 *   const log = createLogger('SyncManager');
 *   log.info('Starting sync');
 *   log.debug('Processing item', { sessionId: '123' });
 *   log.warn('Retrying...', { attempt: 2 });
 *   log.error('Sync failed', new Error('Network error'));
 */

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

interface LoggerConfig {
  /** Minimum log level to output */
  minLevel: LogLevel;
  /** Whether logging is enabled */
  enabled: boolean;
  /** Namespace filter (regex pattern, e.g., 'Sync|Session') */
  filter: string | null;
}

const LOG_LEVELS: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

/**
 * Global logger configuration
 */
const config: LoggerConfig = {
  minLevel: 'info',
  enabled: false,
  filter: null,
};

/**
 * Initialize logger configuration from environment
 */
function initConfig(): void {
  if (typeof window === 'undefined') {
    // Node.js environment - enable for tests
    config.enabled = process.env.NODE_ENV !== 'production';
    config.minLevel = 'debug';
    return;
  }

  try {
    const params = new URLSearchParams(window.location.search);

    // Check URL params first (highest priority)
    if (params.has('log')) {
      config.enabled = true;
      const level = params.get('log');
      if (level && level in LOG_LEVELS) {
        config.minLevel = level as LogLevel;
      }
    }

    if (params.has('logFilter')) {
      config.filter = params.get('logFilter');
    }

    // Check localStorage (second priority)
    if (!config.enabled && typeof localStorage !== 'undefined') {
      const stored = localStorage.getItem('docsign:log');
      if (stored === '1' || stored === 'true') {
        config.enabled = true;
      }
      const storedLevel = localStorage.getItem('docsign:logLevel');
      if (storedLevel && storedLevel in LOG_LEVELS) {
        config.minLevel = storedLevel as LogLevel;
      }
      const storedFilter = localStorage.getItem('docsign:logFilter');
      if (storedFilter) {
        config.filter = storedFilter;
      }
    }

    // Default: enable in development
    if (!config.enabled && process.env.NODE_ENV !== 'production') {
      config.enabled = true;
    }
  } catch {
    // Ignore errors in restricted contexts
  }
}

// Initialize on module load
initConfig();

/**
 * Format log arguments for output
 */
function formatArgs(args: unknown[]): unknown[] {
  return args.map((arg) => {
    if (arg instanceof Error) {
      return {
        name: arg.name,
        message: arg.message,
        stack: arg.stack,
      };
    }
    return arg;
  });
}

/**
 * Check if namespace passes the filter
 */
function passesFilter(namespace: string): boolean {
  if (!config.filter) return true;
  try {
    const regex = new RegExp(config.filter, 'i');
    return regex.test(namespace);
  } catch {
    return true;
  }
}

/**
 * Logger interface for a specific namespace
 */
export interface Logger {
  debug(message: string, ...args: unknown[]): void;
  info(message: string, ...args: unknown[]): void;
  warn(message: string, ...args: unknown[]): void;
  error(message: string, ...args: unknown[]): void;
  /** Get the namespace of this logger */
  readonly namespace: string;
}

/**
 * Create a logger instance for a namespace
 *
 * @param namespace - The namespace/category for this logger (e.g., 'SyncManager')
 * @returns Logger instance with debug, info, warn, error methods
 */
export function createLogger(namespace: string): Logger {
  const prefix = `[${namespace}]`;

  const shouldLog = (level: LogLevel): boolean => {
    if (!config.enabled) return false;
    if (LOG_LEVELS[level] < LOG_LEVELS[config.minLevel]) return false;
    if (!passesFilter(namespace)) return false;
    return true;
  };

  return {
    namespace,

    debug(message: string, ...args: unknown[]): void {
      if (!shouldLog('debug')) return;
      console.debug(prefix, message, ...formatArgs(args));
    },

    info(message: string, ...args: unknown[]): void {
      if (!shouldLog('info')) return;
      console.info(prefix, message, ...formatArgs(args));
    },

    warn(message: string, ...args: unknown[]): void {
      if (!shouldLog('warn')) return;
      console.warn(prefix, message, ...formatArgs(args));
    },

    error(message: string, ...args: unknown[]): void {
      if (!shouldLog('error')) return;
      console.error(prefix, message, ...formatArgs(args));
    },
  };
}

/**
 * Enable logging programmatically
 */
export function enableLogging(level: LogLevel = 'info', filter?: string): void {
  config.enabled = true;
  config.minLevel = level;
  if (filter !== undefined) {
    config.filter = filter;
  }

  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('docsign:log', '1');
      localStorage.setItem('docsign:logLevel', level);
      if (filter) {
        localStorage.setItem('docsign:logFilter', filter);
      }
    }
  } catch {
    // Ignore errors
  }
}

/**
 * Disable logging programmatically
 */
export function disableLogging(): void {
  config.enabled = false;

  try {
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem('docsign:log');
      localStorage.removeItem('docsign:logLevel');
      localStorage.removeItem('docsign:logFilter');
    }
  } catch {
    // Ignore errors
  }
}

/**
 * Get current logging configuration
 */
export function getLogConfig(): Readonly<LoggerConfig> {
  return { ...config };
}

// Pre-created loggers for common namespaces
export const loggers = {
  DocSign: createLogger('DocSign'),
  SyncManager: createLogger('SyncManager'),
  LocalSessionManager: createLogger('LocalSessionManager'),
  CryptoUtils: createLogger('CryptoUtils'),
  PdfLoader: createLogger('PdfLoader'),
  Perf: createLogger('Perf'),
} as const;

// Expose on window for debugging
if (typeof window !== 'undefined') {
  (window as unknown as {
    DocSignLog: {
      enable: typeof enableLogging;
      disable: typeof disableLogging;
      config: typeof getLogConfig;
      create: typeof createLogger;
    };
  }).DocSignLog = {
    enable: enableLogging,
    disable: disableLogging,
    config: getLogConfig,
    create: createLogger,
  };
}
