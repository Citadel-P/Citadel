import type { DashboardWidget } from '@/api/generated/api.types';

export function nextPosition(widgets: DashboardWidget[], width: number, height: number) {
  for (let y = 0; y <= 512; y++)
    for (let x = 0; x <= 12 - width; x++) {
      if (widgets.every(({ layout: l }) => x + width <= l.x || l.x + l.w <= x || y + height <= l.y || l.y + l.h <= y))
        return { x, y, w: width, h: height };
    }
}

export const dashboardGrid = { rowHeight: 44, gap: 12 } as const;
