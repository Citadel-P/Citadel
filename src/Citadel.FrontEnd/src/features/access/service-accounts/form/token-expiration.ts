const DAY_IN_MILLISECONDS = 86_400_000;

export const getTokenExpirationPresets = (defaultLifetimeDays: number, maximumLifetimeDays: number) =>
  [...new Set([30, 90, 180, 365, defaultLifetimeDays])]
    .filter((days) => days > 0 && days <= maximumLifetimeDays)
    .sort((left, right) => left - right);

export const isValidTokenExpiration = (
  value: string,
  maximumLifetimeDays: number,
  referenceTime = Date.now(),
) => {
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp)
    && timestamp > referenceTime
    && timestamp <= referenceTime + maximumLifetimeDays * DAY_IN_MILLISECONDS;
};
