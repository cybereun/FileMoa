export function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value < 1024) return `${Math.max(0, value)} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let amount = value;
  let unit = 'B';
  for (const next of units) {
    amount /= 1024;
    unit = next;
    if (amount < 1024) break;
  }
  return `${amount.toFixed(amount >= 10 ? 0 : 1)} ${unit}`;
}

export function basename(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function requestCount(fileCount: number, batchSize = 50): number {
  if (fileCount <= 0 || batchSize <= 0) return 0;
  return Math.ceil(fileCount / batchSize);
}

export function providerStatus(providerId: string, keyPresent: boolean, available: boolean, language: 'ko' | 'en'): string {
  if (providerId === 'ollama') {
    return available ? (language === 'ko' ? 'Ollama 감지됨' : 'Ollama detected') : (language === 'ko' ? 'Ollama가 실행 중이 아닙니다' : 'Ollama is not running');
  }
  return keyPresent ? (language === 'ko' ? 'API 키 저장됨' : 'API key saved') : (language === 'ko' ? 'API 키 없음' : 'API key not saved');
}

export function shouldOpenUpdateModal(manualCheck: boolean, updateAvailable: boolean): boolean {
  return manualCheck && updateAvailable;
}

export function shouldUseStartupPath(path: string | null | undefined): path is string {
  return typeof path === 'string' && path.trim().length > 0;
}

type Language = 'ko' | 'en';

/** The backend reports a user cancel as an error string; it is not a failure. */
export function isCancelledError(error: unknown): boolean {
  return /취소되었습니다|was cancelled/.test(String(error));
}

export function cloudOnlyNotice(count: number, language: Language): string {
  if (!count) return '';
  return language === 'ko'
    ? `클라우드 전용(온라인 전용) 파일 ${count}개는 다운로드를 막기 위해 검사에서 제외했습니다.`
    : `${count} cloud-only file(s) were skipped so they are not downloaded.`;
}

export type RunCounts = { status: string; executedCount: number; undoneCount: number };

/** Title line of a history card: what moved, what was undone, whether it was cut short. */
export function runTitle(run: RunCounts, language: Language): string {
  const ko = language === 'ko';
  const interrupted = run.status === 'interrupted' ? (ko ? ' · 중단됨' : ' · interrupted') : '';
  if (run.executedCount === 0 && run.undoneCount > 0) {
    return (ko ? `${run.undoneCount}개 모두 되돌림` : `${run.undoneCount} moved, all undone`) + interrupted;
  }
  const moved = ko ? `${run.executedCount}개 이동` : `${run.executedCount} moved`;
  const undone = run.undoneCount > 0 ? (ko ? ` · ${run.undoneCount}개 되돌림` : ` · ${run.undoneCount} undone`) : '';
  return moved + undone + interrupted;
}

/** 0-100 when the total is known, otherwise null (an indeterminate bar). */
export function progressPercent(done: number, total: number): number | null {
  if (!(total > 0)) return null;
  return Math.min(100, Math.round((done / total) * 100));
}
