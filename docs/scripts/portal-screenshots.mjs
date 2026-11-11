// Uses the same isolated API/realtime fixtures as the README capture workflow.
// CITADEL_SCREENSHOT_OUTPUT=/tmp/citadel-portal node docs/scripts/capture-guide-screenshots.mjs portal
import fs from "node:fs/promises";
import path from "node:path";
import { saveScreenshot } from "./screenshot-output.mjs";

export async function capturePortal({ page, origin, expect, data }) {
  const output = process.env.CITADEL_SCREENSHOT_OUTPUT;
  if (!output) throw new Error("Set CITADEL_SCREENSHOT_OUTPUT to the capture directory.");
  await fs.mkdir(output, { recursive: true });
  await page.setViewportSize({ width: 1920, height: 1080 });
  for (const theme of ["light", "dark"]) {
    data.preferences.theme = theme === "dark" ? "Dark" : "Light";
    for (const view of ["platforms", "workloads", "stacks", "networks"]) {
      if (page.url() !== "about:blank")
        await page.evaluate(() => localStorage.removeItem("sidebar-platform-id"));
      const route = view === "platforms" ? "/platforms"
        : view === "stacks" ? "/stacks" : `/stacks/edit/${data.stack.id}/`;
      await page.goto(origin + route, { waitUntil: "networkidle" });
      await expect(page.getByText(view === "platforms" ? "Production Docker" : "Storefront", { exact: true }).first()).toBeVisible();
      if (view === "workloads") {
        await page.getByRole("tab", { name: "Services", exact: true }).click();
        await expect(page.getByText("Ready to accept connections tcp", { exact: false })).toBeVisible();
      }
      if (view === "networks") {
        await page.getByRole("button", { name: "View connections", exact: true }).click();
        const sheet = page.getByRole("dialog");
        await sheet.getByRole("button", { name: "Maximize network sheet" }).click();
        await sheet.getByRole("button", { name: "Expand Storefront", exact: true }).click();
        await expect(sheet.locator(".react-flow__edge")).toHaveCount(3);
        await sheet.getByRole("button", { name: "Show details for storefront_backend", exact: true }).click();
        await expect(sheet.getByText("172.21.0.1", { exact: true })).toBeVisible();
      }
      await expect(page.locator("html")).toHaveClass(new RegExp(theme));
      await expect(page.locator("html")).toHaveAttribute("data-theme-color", "blue");
      await expect(page.getByText("Live updates unavailable", { exact: false })).toHaveCount(0);
      await page.evaluate(() => document.fonts.ready);
      await expect(page.locator('div[style*="position: fixed"][style*="height: 2px"] > div').first()).toHaveCSS("width", "0px");
      await page.mouse.move(0, 0);
      const basename = `${view}-blue-${theme}`;
      const pngPath = path.join(output, `${basename}.png`);
      await saveScreenshot(page, { path: pngPath });
      const source = `data:image/png;base64,${(await fs.readFile(pngPath)).toString("base64")}`;
      // Encode and downsample the real capture; do not alter the product UI.
      for (const width of [960, 1920]) {
        const encoded = await page.evaluate(async ({ source, width }) => {
          const image = new Image();
          image.src = source;
          await image.decode();
          const canvas = document.createElement("canvas");
          canvas.width = width;
          canvas.height = width * image.height / image.width;
          canvas.getContext("2d").drawImage(image, 0, 0, canvas.width, canvas.height);
          return canvas.toDataURL("image/webp", 0.94).split(",")[1];
        }, { source, width });
        await fs.writeFile(path.join(output, `${basename}-${width}.webp`), Buffer.from(encoded, "base64"));
      }
      console.log(`Captured ${basename}`);
    }
  }
}
