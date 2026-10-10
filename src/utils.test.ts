import { describe, expect, it } from 'vitest';
import { basename, cloudOnlyNotice, decideOpenFolder, duplicateGroupTitle, planSummary, formatBytes, isCancelledError, progressPercent, providerStatus, requestCount, runTitle, shouldOpenUpdateModal, shouldUseStartupPath } from './utils';

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

  it('accepts only a non-empty shell startup path', () => {
    expect(shouldUseStartupPath(String.raw`C:\Work\Files`)).toBe(true);
    expect(shouldUseStartupPath('   ')).toBe(false);
    expect(shouldUseStartupPath(null)).toBe(false);
  });

  it('treats the backend cancel message as a cancel, not a failure', () => {
    expect(isCancelledError('작업이 취소되었습니다. / The operation was cancelled.')).toBe(true);
    expect(isCancelledError(new Error('The operation was cancelled.'))).toBe(true);
    expect(isCancelledError('다른 작업이 진행 중입니다.')).toBe(false);
    expect(isCancelledError('disk full')).toBe(false);
  });

  it('only mentions cloud-only files when some were skipped', () => {
    expect(cloudOnlyNotice(0, 'ko')).toBe('');
    expect(cloudOnlyNotice(3, 'ko')).toContain('3개');
    expect(cloudOnlyNotice(3, 'en')).toBe('3 cloud-only file(s) were skipped so they are not downloaded.');
  });

  it('describes a history card in both languages', () => {
    expect(runTitle({ status: 'completed', executedCount: 14, undoneCount: 0 }, 'ko')).toBe('14개 이동');
    expect(runTitle({ status: 'completed', executedCount: 3, undoneCount: 11 }, 'ko')).toBe('3개 이동 · 11개 되돌림');
    expect(runTitle({ status: 'completed', executedCount: 0, undoneCount: 14 }, 'ko')).toBe('14개 모두 되돌림');
    expect(runTitle({ status: 'completed', executedCount: 0, undoneCount: 14 }, 'en')).toBe('14 moved, all undone');
    expect(runTitle({ status: 'interrupted', executedCount: 1, undoneCount: 0 }, 'en')).toBe('1 moved · interrupted');
    expect(runTitle({ status: 'interrupted', executedCount: 1, undoneCount: 0 }, 'ko')).toBe('1개 이동 · 중단됨');
  });

  it('computes progress only when the total is known', () => {
    expect(progressPercent(0, 0)).toBeNull();
    expect(progressPercent(5, 0)).toBeNull();
    expect(progressPercent(50, 200)).toBe(25);
    expect(progressPercent(300, 200)).toBe(100);
    expect(progressPercent(0, 10)).toBe(0);
  });

  it('writes the plan summary in the UI language', () => {
    const counts = { files: 420, exclusions: 2, errors: 0, warnings: 0 };
    expect(planSummary(counts, 'ko')).toBe('420개 파일 · 제외 2개 · 오류 0개');
    expect(planSummary(counts, 'en')).toBe('420 files · 2 exclusions · 0 errors');
    expect(planSummary({ ...counts, warnings: 3 }, 'ko')).toBe('420개 파일 · 제외 2개 · 오류 0개 · 주의 3개');
    expect(planSummary({ ...counts, warnings: 3 }, 'en')).toBe('420 files · 2 exclusions · 0 errors · 3 warnings');
  });

  it('titles a duplicate group with the space the extra copies take', () => {
    expect(duplicateGroupTitle(3, 8 * 1024 * 1024, 'ko')).toBe('3개 파일 · 8.0 MB 절약 가능');
    expect(duplicateGroupTitle(3, 8 * 1024 * 1024, 'en')).toBe('3 files · 8.0 MB reclaimable');
  });

  it('decides what a folder handed over by a second launch does', () => {
    const idle = { busy: false, hasPreview: false };
    expect(decideOpenFolder({}, idle)).toBe('bring-forward');
    expect(decideOpenFolder({ path: null, error: null }, { busy: true, hasPreview: true })).toBe('bring-forward');
    expect(decideOpenFolder({ error: 'folder is gone' }, idle)).toBe('error');
    expect(decideOpenFolder({ path: 'C:\\Photos' }, idle)).toBe('open');
    expect(decideOpenFolder({ path: 'C:\\Photos' }, { busy: false, hasPreview: true })).toBe('confirm');
    // A running operation wins over everything else about the window.
    expect(decideOpenFolder({ path: 'C:\\Photos' }, { busy: true, hasPreview: true })).toBe('busy');
    expect(decideOpenFolder({ path: 'C:\\Photos' }, { busy: true, hasPreview: false })).toBe('busy');
  });
});
