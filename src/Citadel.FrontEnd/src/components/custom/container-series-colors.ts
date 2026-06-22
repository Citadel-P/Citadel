export const CONTAINER_SERIES_COLORS = [
  { text: 'text-sky-400', dot: 'bg-sky-400', stroke: 'hsl(199 89% 48%)' },
  { text: 'text-emerald-400', dot: 'bg-emerald-400', stroke: 'hsl(160 84% 39%)' },
  { text: 'text-amber-400', dot: 'bg-amber-400', stroke: 'hsl(38 92% 50%)' },
  { text: 'text-rose-400', dot: 'bg-rose-400', stroke: 'hsl(349 89% 60%)' },
  { text: 'text-violet-400', dot: 'bg-violet-400', stroke: 'hsl(258 90% 66%)' },
  { text: 'text-cyan-400', dot: 'bg-cyan-400', stroke: 'hsl(188 86% 43%)' },
  { text: 'text-lime-400', dot: 'bg-lime-400', stroke: 'hsl(82 84% 44%)' },
  { text: 'text-orange-400', dot: 'bg-orange-400', stroke: 'hsl(24 95% 53%)' },
];

export const getContainerSeriesColor = (containerName: string) => {
  let hash = 0;
  for (let i = 0; i < containerName.length; i++) {
    hash = (hash * 31 + containerName.charCodeAt(i)) >>> 0;
  }

  return CONTAINER_SERIES_COLORS[hash % CONTAINER_SERIES_COLORS.length];
};
