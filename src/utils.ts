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
