import type { AppError } from "@shared/types";

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null;
}

/**
 * Normalizes errors coming from Tauri `invoke()` into a user-friendly string.
 *
 * Tauri errors can be:
 * - a plain string
 * - an Error instance
 * - a serialized Rust `AppError` object (`{ kind, message }`)
 * - a nested wrapper such as `{ error: ... }`
 */
export function normalizeTauriError(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;

  const tryAppError = (v: unknown): AppError | null => {
    if (!isRecord(v)) return null;
    const kind = v.kind;
    const message = v.message;
    if (typeof kind === "string" && typeof message === "string") {
      return { kind: kind as AppError["kind"], message };
    }
    return null;
  };

  const direct = tryAppError(err);
  if (direct) return `${direct.kind}: ${direct.message}`;

  if (isRecord(err) && "error" in err) {
    const nested = tryAppError(err.error);
    if (nested) return `${nested.kind}: ${nested.message}`;
    return normalizeTauriError(err.error);
  }

  try {
    return JSON.stringify(err);
  } catch {
    return String(err);
  }
}

function isLinux() {
  return navigator.userAgent.toLowerCase().includes("linux");
}

/**
 * Adds a Linux-specific setup hint for common permission/bootstrap failures.
 */
export function withPlatformSetupHint(message: string): string {
  if (!isLinux()) return message;
  const m = message.toLowerCase();
  const needsHint =
    m.includes("udev") ||
    m.includes("pkexec") ||
    m.includes("permission") ||
    m.includes("not prepared");
  if (!needsHint) return message;
  return `${message}\n\nHint: ensure udev rules are installed and administrator authorization was granted, then reconnect the probe and retry.`;
}

