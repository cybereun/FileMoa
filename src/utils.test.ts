import { describe, expect, it } from 'vitest';
import { basename, formatBytes, providerStatus, requestCount, shouldOpenUpdateModal } from './utils';

describe('FileMoa UI utilities', () => {
  it('formats byte counts for the large-file view', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1024 * 1024)).toBe('1.0 MB');
    expect(formatBytes(2 * 1024 * 1024 * 1024)).toBe('2.0 GB');
  });

  it('extracts a basename without exposing its parent path', () => {
    expect(basename(String.raw`C:\Users\demo\receipt.pdf`)).toBe('receipt.pdf');
    expect(basename('/tmp/photo.png')).toBe('photo.png');
  });

  it('counts the bounded metadata requests', () => {
    expect(requestCount(0)).toBe(0);
    expect(requestCount(51)).toBe(2);
    expect(requestCount(120, 25)).toBe(5);
  });

  it('keeps provider key state masked and language-aware', () => {
    expect(providerStatus('openai', true, false, 'ko')).toBe('API 키 저장됨');
    expect(providerStatus('openai', false, false, 'en')).toBe('API key not saved');
    expect(providerStatus('ollama', false, true, 'en')).toBe('Ollama detected');
  });

  it('opens an update modal only for a manual check with a newer release', () => {
    expect(shouldOpenUpdateModal(false, true)).toBe(false);
    expect(shouldOpenUpdateModal(true, false)).toBe(false);
    expect(shouldOpenUpdateModal(true, true)).toBe(true);
  });
});
