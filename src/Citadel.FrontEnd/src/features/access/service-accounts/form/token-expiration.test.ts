import { getTokenExpirationPresets, isValidTokenExpiration } from './token-expiration';

describe('Service Account token expiration', () => {
  it('includes the configured default and excludes presets beyond the license maximum', () => {
    expect(getTokenExpirationPresets(60, 180)).toEqual([30, 60, 90, 180]);
    expect(getTokenExpirationPresets(90, 30)).toEqual([30]);
  });

  it('accepts only future dates within the configured maximum lifetime', () => {
    const now = Date.parse('2026-08-12T12:00:00.000Z');

    expect(isValidTokenExpiration('2026-08-13T12:00:00.000Z', 30, now)).toBe(true);
    expect(isValidTokenExpiration('2026-08-12T12:00:00.000Z', 30, now)).toBe(false);
    expect(isValidTokenExpiration('2026-09-12T12:00:00.000Z', 30, now)).toBe(false);
    expect(isValidTokenExpiration('not-a-date', 30, now)).toBe(false);
  });
});
