export const NETWORK_CHART_COLORS = {
  rxBytes: {
    light: 'hsl(199 89% 48%)',
    dark: 'hsl(199 95% 64%)',
  },
  txBytes: {
    light: 'hsl(38 92% 50%)',
    dark: 'hsl(43 96% 56%)',
  },
} as const;

export type ThemedChartColor = (typeof NETWORK_CHART_COLORS)[keyof typeof NETWORK_CHART_COLORS];
