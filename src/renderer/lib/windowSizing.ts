import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

const BODY_PADDING = 24; // 12px top + 12px bottom (matches body padding in styles.css)

// Resize only the window HEIGHT to fit content; preserves whatever width the user has set.
export async function resizeHeightToContent() {
  await new Promise<void>((r) => setTimeout(r, 80)); // wait one paint for DOM to settle
  const card =
    document.querySelector<HTMLElement>(".app-card") ??
    document.querySelector<HTMLElement>(".message-card") ??
    document.querySelector<HTMLElement>(".bootstrap-lite-card");
  if (!card) return;

  const targetHeight = card.scrollHeight + BODY_PADDING;
  try {
    const win = getCurrentWindow();
    const [physicalSize, scale] = await Promise.all([win.outerSize(), win.scaleFactor()]);
    const currentLogicalWidth = Math.round(physicalSize.width / scale);
    await win.setSize(new LogicalSize(currentLogicalWidth, targetHeight));
  } catch (e) {
    console.error("[windowSizing] window resize failed:", e);
  }
}

