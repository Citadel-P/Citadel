import DOMPurify from 'dompurify';
import sanitizer from './sanitizer';

describe('Monaco sanitizer', () => {
  it('preserves editor markup while removing executable attributes and URLs', () => {
    const result = sanitizer.sanitize(
      '<p><strong>Documentation</strong><code>echo hello</code>' +
        '<a href="javascript:alert(1)">link</a><img src="x" onerror="alert(1)"><script>alert(1)</script></p>',
    );
    const element = document.createElement('div');
    element.innerHTML = result;

    expect(element.querySelector('strong')).toHaveTextContent('Documentation');
    expect(element.querySelector('code')).toHaveTextContent('echo hello');
    expect(element.querySelector('a')).not.toHaveAttribute('href');
    expect(element.querySelector('img')).not.toHaveAttribute('onerror');
    expect(element.querySelector('script')).toBeNull();
  });

  it('does not remove hooks belonging to other sanitizer consumers', () => {
    const hook = vi.fn();
    DOMPurify.addHook('afterSanitizeAttributes', hook);
    try {
      sanitizer.removeAllHooks();
      DOMPurify.sanitize('<strong>text</strong>');
      expect(hook).toHaveBeenCalled();
      expect(sanitizer).not.toBe(DOMPurify);
    } finally {
      DOMPurify.removeHook('afterSanitizeAttributes', hook);
    }
  });
});
