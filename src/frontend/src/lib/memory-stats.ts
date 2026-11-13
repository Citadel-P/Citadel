type MemoryValues = Partial<Record<'memoryActive' | 'memoryActiveCache' | 'memoryCache', number | null>>;

export function memoryBreakdown<T extends MemoryValues>(stat: T) {
  const active = stat.memoryActiveCache;
  const working = stat.memoryActive;
  const inactive = stat.memoryCache;
  const known =
    active != null &&
    working != null &&
    inactive != null &&
    Number.isFinite(active) &&
    Number.isFinite(working) &&
    Number.isFinite(inactive) &&
    active >= 0 &&
    inactive >= 0 &&
    active <= working;
  return {
    ...stat,
    nonCache: known ? working - active : null,
    activeCache: known ? active : null,
    // Do not draw a partial stack as though it were the total.
    inactiveCache: known ? inactive : null,
  };
}
