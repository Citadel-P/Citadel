const trail = '...';

export function truncate(value: string, limit?: number, dir?: 'left' | 'right'): string {
  if (limit && value.length < limit) return value;

  limit = limit ?? 18;
  dir = dir ?? 'right';
  if (dir === 'left') {
    return trail + value.substring(value.length, value.length - limit);
  }
  return value.substring(0, limit) + trail;
}
