function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return `${error.name}: ${error.message}`;
  }
  if (typeof error === "string") {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

export function logRendererError(scope: string, action: string, error: unknown) {
  console.error(`[renderer][${scope}] ${action} failed: ${toErrorMessage(error)}`);
}

export function logRendererInfo(scope: string, message: string) {
  console.info(`[renderer][${scope}] ${message}`);
}
