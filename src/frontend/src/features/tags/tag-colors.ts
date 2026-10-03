type TagColorFamily = {
  name: string;
  light: string;
  base: string;
  dark: string;
};

function capitalize(value: string) {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

const TAG_COLOR_FAMILIES: TagColorFamily[] = [
  { name: 'slate', light: '#f1f5f9', base: '#94a3b8', dark: '#1e293b' },
  { name: 'gray', light: '#f3f4f6', base: '#9ca3af', dark: '#1f2937' },
  { name: 'zinc', light: '#f4f4f5', base: '#a1a1aa', dark: '#27272a' },
  { name: 'neutral', light: '#f5f5f5', base: '#a3a3a3', dark: '#262626' },
  { name: 'stone', light: '#f5f5f4', base: '#a8a29e', dark: '#292524' },
  { name: 'red', light: '#fee2e2', base: '#f87171', dark: '#991b1b' },
  { name: 'orange', light: '#ffedd5', base: '#fb923c', dark: '#9a3412' },
  { name: 'amber', light: '#fef3c7', base: '#fbbf24', dark: '#92400e' },
  { name: 'yellow', light: '#fef9c3', base: '#facc15', dark: '#854d0e' },
  { name: 'lime', light: '#ecfccb', base: '#a3e635', dark: '#3f6212' },
  { name: 'green', light: '#dcfce7', base: '#4ade80', dark: '#166534' },
  { name: 'emerald', light: '#d1fae5', base: '#34d399', dark: '#065f46' },
  { name: 'teal', light: '#ccfbf1', base: '#2dd4bf', dark: '#115e59' },
  { name: 'cyan', light: '#cffafe', base: '#22d3ee', dark: '#155e75' },
  { name: 'sky', light: '#e0f2fe', base: '#38bdf8', dark: '#075985' },
  { name: 'blue', light: '#dbeafe', base: '#60a5fa', dark: '#1e40af' },
  { name: 'indigo', light: '#e0e7ff', base: '#818cf8', dark: '#3730a3' },
  { name: 'violet', light: '#ede9fe', base: '#a78bfa', dark: '#5b21b6' },
  { name: 'purple', light: '#f3e8ff', base: '#c084fc', dark: '#6b21a8' },
  { name: 'fuchsia', light: '#fae8ff', base: '#e879f9', dark: '#86198f' },
  { name: 'pink', light: '#fce7f3', base: '#f472b6', dark: '#9d174d' },
  { name: 'rose', light: '#ffe4e6', base: '#fb7185', dark: '#9f1239' },
];

const SHADE_LABELS = {
  light: 'Light',
  base: '',
  dark: 'Dark',
} as const;

export type TagColorOption = {
  value: string;
  label: string;
  family: string;
  shade: keyof typeof SHADE_LABELS;
  textColor: string;
};

export const TAG_COLOR_OPTIONS: TagColorOption[] = TAG_COLOR_FAMILIES.flatMap((family) =>
  (['light', 'base', 'dark'] as const).map((shade) => ({
    value: family[shade],
    label: [SHADE_LABELS[shade], capitalize(family.name)].filter(Boolean).join(' '),
    family: family.name,
    shade,
    textColor: shade === 'dark' ? '#ffffff' : '#0f172a',
  })),
);

export const DEFAULT_TAG_COLOR = TAG_COLOR_OPTIONS.find((option) => option.label === 'Slate') ?? TAG_COLOR_OPTIONS[0];

export const getTagColorOption = (color: string | null | undefined): TagColorOption | undefined => {
  if (!color) return undefined;
  const normalized = color.toLowerCase();
  return TAG_COLOR_OPTIONS.find((option) => option.value.toLowerCase() === normalized);
};

export const getTagColorLabel = (color: string | null | undefined) => getTagColorOption(color)?.label ?? 'Custom';

export const getTagTextColor = (color: string | null | undefined) => getTagColorOption(color)?.textColor ?? '#ffffff';
