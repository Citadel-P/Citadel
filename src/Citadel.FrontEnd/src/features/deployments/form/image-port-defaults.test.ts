import { shouldApplyLocalImagePortDefaults } from './image-port-defaults';

describe('shouldApplyLocalImagePortDefaults', () => {
  it('does not alter ports during initial form load', () => {
    expect(shouldApplyLocalImagePortDefaults(null, 'image-id', [])).toBe(false);
  });

  it('applies exposed ports after an explicit image selection', () => {
    expect(shouldApplyLocalImagePortDefaults('image-id', 'image-id', [])).toBe(true);
  });

  it('does not overwrite ports edited while image defaults are loading', () => {
    expect(shouldApplyLocalImagePortDefaults('image-id', 'image-id', ['8080:80/tcp'])).toBe(false);
  });

  it('ignores a response for a previously selected image', () => {
    expect(shouldApplyLocalImagePortDefaults('old-image', 'new-image', [])).toBe(false);
  });
});
