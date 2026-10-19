export const MIN_SCREENSHOT_WIDTH = 1660;

// Render at the requested resolution; never enlarge an existing bitmap.
export async function saveScreenshot(target, options) {
  const png = await target.screenshot({ animations: "disabled", ...options });
  const width = png.readUInt32BE(16);
  if (width < MIN_SCREENSHOT_WIDTH) {
    throw new Error(
      `${options.path}: screenshot width ${width}px is below ${MIN_SCREENSHOT_WIDTH}px`,
    );
  }
}
