const trail = '...';

export function truncate(value: string, limit?: number, dir?: 'left' | 'right', noTrail?: boolean): string {
  if (limit && value.length < limit) return value;

  limit = limit ?? 18;
  dir = dir ?? 'right';
  if (dir === 'left') {
    return noTrail
      ? value.substring(value.length, value.length - limit)
      : trail + value.substring(value.length, value.length - limit);
  }
  return noTrail ? value.substring(0, limit) : value.substring(0, limit) + trail;
}
