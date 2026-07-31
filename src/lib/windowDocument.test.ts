import { describe, expect, it } from 'vitest';
import { markWindowDocument } from './windowDocument';

describe('markWindowDocument', () => {
  it('marks the companion document before its transparent UI renders', () => {
    const root = document.createElement('html');

    markWindowDocument('companion', root);

    expect(root.dataset.window).toBe('companion');
  });

  it('keeps the main document distinguishable from the companion', () => {
    const root = document.createElement('html');

    markWindowDocument('main', root);

    expect(root.dataset.window).toBe('main');
  });
});
