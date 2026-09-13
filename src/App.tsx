import { useEffect, useMemo, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { check, type Update } from '@tauri-apps/plugin-updater';
import {
  Bot,
  CheckCircle2,
  Clock3,
  Copy,
  Download,
  ExternalLink,
  Files,
  FolderOpen,
  FolderTree,
  History,
  Play,
  RefreshCw,
  Search,
  Settings,
  ShieldCheck,
  Tags,
  Trash2,
  Undo2,
  X,
} from 'lucide-react';
import './styles.css';
import { basename, formatBytes, providerStatus, requestCount, shouldOpenUpdateModal, shouldUseStartupPath } from './utils';

type Category = { name: string; extensions: string[]; enabled: boolean };
type KeywordRule = { keyword: string; category: string; caseSensitive?: boolean };
type Action = {
  sourcePath: string;
  destinationPath: string;
  category: string;
  reason: string;
  status: string;
  sizeBytes?: number;
  modifiedUnixSecs?: number | null;
  collision?: boolean;
};
type Plan = {
  rootPath: string;
  actions: Action[];
  exclusions: string[];
  errors: string[];
  warnings?: string[];
  createdAt?: string;
  basis?: string;
  language?: string;
  mode?: string;
  rootPaths?: string[];
};
type Duplicate = {
  hash: string;
  sizeBytes: number;
  filePaths: string[];
  reclaimableBytes: number;
};
type LargeFile = { path: string; sizeBytes: number };
type KnownFolder = { id: string; path: string };
type AiProvider = {
  id: string;
  displayName: string;
  model: string;
  keyPresent: boolean;
  enabled: boolean;
  registrationUrl: string;
  endpoint: string;
  ollamaModels: string[];
  available: boolean;
};
type AiSuggestion = {
  id: number;
  category: string;
  confidence: number;
  reason: string;
  isNewCategory: boolean;
  needsReview: boolean;
};
type AiResult = {
  providerId: string;
  model: string;
  requestCount: number;
  suggestions: AiSuggestion[];
  proposedCategories: string[];
};
type Tab = 'organize' | 'manage' | 'safety';
type Panel = 'plan' | 'categories' | 'duplicates' | 'history' | 'large' | 'empty' | 'ai';

const labels = {
  ko: {
    organize: '정리', manage: '관리', safety: '안전', settings: '설정', folder: '폴더 선택',
    preview: '정리 미리보기', execute: '파일 이동', category: '사용자 카테고리', duplicates: '중복 파일 탐색',
    undo: '마지막 정리 되돌리기', history: '정리 기록', aiSettings: 'AI 설정', sub: '하위 폴더 포함',
    empty: '정리할 폴더를 선택하세요', nothing: '표시할 항목이 없습니다.', update: '업데이트 확인', language: 'English',
    catTitle: '사용자 정의 카테고리', catHint: '이름과 확장자를 입력하세요. 예: 영수증 / pdf,xlsx', add: '카테고리 추가',
    scan: '중복 탐색 실행', large: '대용량 파일', emptyFolders: '빈 폴더', plan: '정리 예정', aiClassify: 'AI 분류', aiTitle: 'AI 파일 분류 설정', aiResults: 'AI 분류 결과',
    aiHint: 'AI는 사용자가 버튼을 눌렀을 때만 파일명·메타데이터를 전송합니다. 파일 내용과 경로는 전송하지 않습니다.',
    selected: '이번 분류에 사용할 공급자', save: '설정 저장', test: '연결 테스트', removeKey: '저장된 키 삭제',
    apiKey: 'API 키', model: '모델 ID', modelHint: '권장값이 기본으로 입력됩니다. 필요하면 직접 변경할 수 있습니다.',
    getKey: '공식 키 발급 주소', localOnly: '키는 이 Windows 계정의 암호화 저장소에만 보관됩니다.',
    ollamaReady: 'Ollama 감지됨', ollamaMissing: 'Ollama가 실행 중이 아닙니다', ollamaModels: '설치된 모델',
    updateTitle: 'FileMoa 업데이트', currentVersion: '현재 버전', newVersion: '새 버전', installNow: '지금 설치',
    installLater: '나중에 설치', latest: '최신 버전을 사용하고 있습니다.', updateAvailable: '새 업데이트가 준비되었습니다.',
    updateLoading: '업데이트 정보를 확인하는 중…', updateInstalling: '업데이트를 다운로드하고 설치하는 중…', retry: '다시 시도',
    runAi: 'AI 분류 실행', reviewRequired: '검토 필요', newCategory: '새 카테고리 제안', requestCount: '요청 수', applySuggestions: '선택한 제안을 계획에 반영',
    basis: '분류 기준', byType: '파일 형식', byDate: '날짜', byYear: '연도', bySize: '파일 크기', byName: '파일명 규칙', dateBasis: '날짜 종류', byCreated: '생성 날짜', byModified: '수정 날짜', byAccessed: '마지막 사용', quickFolders: '빠른 폴더', desktop: '바탕화면', downloads: '다운로드', documents: '문서', pictures: '사진', videos: '동영상', music: '음악', includeHidden: '숨김 파일 포함', multiRoot: '여러 폴더', mode: '정리 모드', safeMode: '안전', normalMode: '일반', strongMode: '강력', exceptions: '예외 목록', exceptionHint: '폴더 경로 또는 확장자를 입력하면 정리에서 제외합니다.', addException: '예외 추가',
    errors: '오류', warnings: '주의', status: '상태', collision: '이름 충돌', select: '선택', recycle: '휴지통으로 보내기', emptyHint: '삭제하지 않고 후보만 표시합니다.', largeHint: '크기순으로 표시합니다. 삭제하지 않습니다.', largeThreshold: '최소 크기(MB)',
    undoPreview: '되돌리기 미리보기', undoConfirm: '선택한 정리를 되돌리기', cancel: '취소', moved: '이동됨', skipped: '건너뜀', failed: '실패', undone: '되돌림',
  },
  en: {
    organize: 'Organize', manage: 'Manage', safety: 'Safety', settings: 'Settings', folder: 'Choose folder',
    preview: 'Preview plan', execute: 'Move files', category: 'Custom categories', duplicates: 'Find duplicates',
    undo: 'Undo last run', history: 'Run history', aiSettings: 'AI settings', sub: 'Include subfolders',
    empty: 'Choose a folder to organize', nothing: 'Nothing to show yet.', update: 'Check updates', language: '한국어',
    catTitle: 'Custom categories', catHint: 'Enter a name and extensions. Example: Receipts / pdf,xlsx', add: 'Add category',
    scan: 'Scan duplicates', large: 'Large files', emptyFolders: 'Empty folders', plan: 'Planned changes', aiClassify: 'AI classify', aiTitle: 'AI file classification settings', aiResults: 'AI suggestions',
    aiHint: 'AI sends filenames and metadata only when you press the button. File contents and paths are never sent.',
    selected: 'Provider for the next classification', save: 'Save settings', test: 'Test connection', removeKey: 'Remove saved key',
    apiKey: 'API key', model: 'Model ID', modelHint: 'A recommended value is provided and can be edited in advanced settings.',
    getKey: 'Official key page', localOnly: 'Keys stay in an encrypted store bound to this Windows account.',
    ollamaReady: 'Ollama detected', ollamaMissing: 'Ollama is not running', ollamaModels: 'Installed models',
    updateTitle: 'FileMoa update', currentVersion: 'Current version', newVersion: 'New version', installNow: 'Install now',
    installLater: 'Install later', latest: 'You already have the latest version.', updateAvailable: 'A new update is available.',
    updateLoading: 'Checking for updates…', updateInstalling: 'Downloading and installing the update…', retry: 'Retry',
    runAi: 'Run AI classification', reviewRequired: 'Review required', newCategory: 'New category suggestion', requestCount: 'Requests', applySuggestions: 'Apply selected suggestions to plan',
    basis: 'Classification basis', byType: 'File type', byDate: 'Date', byYear: 'Year', bySize: 'File size', byName: 'Filename rules', dateBasis: 'Date field', byCreated: 'Created', byModified: 'Modified', byAccessed: 'Last accessed', quickFolders: 'Quick folders', desktop: 'Desktop', downloads: 'Downloads', documents: 'Documents', pictures: 'Pictures', videos: 'Videos', music: 'Music', includeHidden: 'Include hidden files', multiRoot: 'Multiple folders', mode: 'Organization mode', safeMode: 'Safe', normalMode: 'Normal', strongMode: 'Strong', exceptions: 'Exclusions', exceptionHint: 'Paths or extensions entered here are excluded from organization.', addException: 'Add exclusion',
    errors: 'Errors', warnings: 'Warnings', status: 'Status', collision: 'Collision', select: 'Select', recycle: 'Move to Recycle Bin', emptyHint: 'Candidates are shown only; nothing is deleted.', largeHint: 'Sorted by size; nothing is deleted.', largeThreshold: 'Minimum size (MB)', undoPreview: 'Undo preview', undoConfirm: 'Undo selected run', cancel: 'Cancel', moved: 'Moved', skipped: 'Skipped', failed: 'Failed', undone: 'Undone',
  },
} as const;

function readLocal<T>(key: string, fallback: T): T {
  try {
    const value = localStorage.getItem(key);
    return value ? (JSON.parse(value) as T) : fallback;
  } catch {
    return fallback;
  }
}

export default function App() {
  const [lang, setLang] = useState<'ko' | 'en'>(() => {
    const stored = localStorage.getItem('filemoa-language');
    if (stored === 'ko' || stored === 'en') return stored;
    return navigator.language.toLowerCase().startsWith('ko') ? 'ko' : 'en';
  });
  const [tab, setTab] = useState<Tab>('organize');
  const [panel, setPanel] = useState<Panel>('plan');
  const [path, setPath] = useState('');
  const [paths, setPaths] = useState<string[]>([]);
  const [multiRoot, setMultiRoot] = useState(false);
  const [sub, setSub] = useState(false);
  const [plan, setPlan] = useState<Plan | null>(null);
  const [runs, setRuns] = useState<Plan[]>(() => {
    const cutoff = Date.now() - 30 * 24 * 60 * 60 * 1000;
    return readLocal<Plan[]>('filemoa-runs', []).filter((run) => {
      if (!run.createdAt) return true;
      const timestamp = Date.parse(run.createdAt);
      return Number.isNaN(timestamp) || timestamp >= cutoff;
    });
  });
  const [cats, setCats] = useState<Category[]>(() => readLocal('filemoa-categories', []));
  const [rules, setRules] = useState<KeywordRule[]>(() => readLocal('filemoa-keyword-rules', []));
  const [excludedPaths, setExcludedPaths] = useState<string[]>(() => readLocal('filemoa-excluded-paths', []));
  const [excludedExtensions, setExcludedExtensions] = useState<string[]>(() => readLocal('filemoa-excluded-extensions', []));
  const [duplicates, setDuplicates] = useState<Duplicate[]>([]);
  const [duplicateSelection, setDuplicateSelection] = useState<string[]>([]);
  const [largeFiles, setLargeFiles] = useState<LargeFile[]>([]);
  const [emptyFolders, setEmptyFolders] = useState<string[]>([]);
  const [cleanupSelection, setCleanupSelection] = useState<string[]>([]);
  const [knownFolders, setKnownFolders] = useState<KnownFolder[]>([]);
  const [basis, setBasis] = useState(() => localStorage.getItem('filemoa-basis') || 'type');
  const [mode, setMode] = useState(() => localStorage.getItem('filemoa-mode') || 'safe');
  const [dateBasis, setDateBasis] = useState(() => localStorage.getItem('filemoa-date-basis') || 'modified');
  const [largeThresholdMb, setLargeThresholdMb] = useState(() => localStorage.getItem('filemoa-large-threshold-mb') || '100');
  const [includeHidden, setIncludeHidden] = useState(false);
  const [undoPreview, setUndoPreview] = useState<Plan | null>(null);
  const [aiProviders, setAiProviders] = useState<AiProvider[]>([]);
  const [selectedAi, setSelectedAi] = useState('ollama');
  const [aiKeyDraft, setAiKeyDraft] = useState('');
  const [aiModelDraft, setAiModelDraft] = useState('');
  const [aiMessage, setAiMessage] = useState('');
  const [aiResult, setAiResult] = useState<AiResult | null>(null);
  const [approvedAiIds, setApprovedAiIds] = useState<number[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const [settings, setSettings] = useState(false);
  const [name, setName] = useState('');
  const [exts, setExts] = useState('');
  const [ruleKeyword, setRuleKeyword] = useState('');
  const [ruleCategory, setRuleCategory] = useState('');
  const [excludedDraft, setExcludedDraft] = useState('');
  const [pendingUpdate, setPendingUpdate] = useState<Update | null>(null);
  const [updateModal, setUpdateModal] = useState(false);
  const [updateProgress, setUpdateProgress] = useState('');
  const t = labels[lang];

  const total = useMemo(() => plan?.actions.length ?? 0, [plan]);
  const selectedProvider = aiProviders.find((provider) => provider.id === selectedAi);

  async function choose() {
    const selected = await open({ directory: true, multiple: multiRoot, title: t.folder });
    const selectedPaths = Array.isArray(selected) ? selected : typeof selected === 'string' ? [selected] : [];
    if (selectedPaths.length) {
      setPaths(selectedPaths);
      setPath(selectedPaths.join(' · '));
      setPlan(null);
      setPanel('plan');
    }
  }

  function chooseKnownFolder(folder: KnownFolder) {
    setPaths([folder.path]);
    setPath(folder.path);
    setPlan(null);
    setPanel('plan');
  }

  function knownFolderLabel(id: string) {
    const key = id as keyof typeof t;
    return key in t ? t[key] : id;
  }

  async function makePlan() {
    if (!path) return;
    setBusy(true);
    try {
      const request = {
          rootPath: path,
          includeSubfolders: sub,
          customCategories: cats,
          language: lang,
          basis,
          mode,
          dateBasis,
          includeHidden,
          excludedPaths,
          excludedExtensions,
          keywordRules: rules,
        };
      const next = paths.length > 1
        ? await invoke<Plan>('plan_multiple_organizations', { requests: paths.map((rootPath) => ({ ...request, rootPath })) })
        : await invoke<Plan>('plan_organization', { request: { ...request, rootPath: paths[0] ?? path } });
      setPlan(next);
      setPanel('plan');
      setNotice('');
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function execute() {
    if (!plan) return;
    const confirmation = total >= 100
      ? (lang === 'ko'
        ? `대량 작업입니다. ${total}개 파일을 이동합니다. 미리보기와 경고를 확인했나요? 계속할까요?`
        : `This is a bulk operation: ${total} files will move. Have you reviewed the preview and warnings? Continue?`)
      : (lang === 'ko' ? `${total}개 파일을 이동할까요?` : `Move ${total} files?`);
    if (!confirm(confirmation)) return;
    setBusy(true);
    try {
      const done = await invoke<Plan>('execute_organization', { plan });
      setPlan(done);
      setRuns((previous) => [done, ...previous].slice(0, 100));
      setPanel('history');
      setNotice(lang === 'ko' ? '정리가 완료되었습니다.' : 'Organization completed.');
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  function openUndoPreview(run = runs[0]) {
    if (run) setUndoPreview(run);
  }

  async function undo(run = undoPreview ?? runs[0]) {
    if (!run) return;
    const restoreCount = run.actions.filter((action) => action.status === 'executed').length;
    if (!restoreCount || !confirm(lang === 'ko' ? `${restoreCount}개 파일을 원래 위치로 되돌릴까요? 기존 파일은 덮어쓰지 않습니다.` : `Restore ${restoreCount} files to their original locations? Existing files will never be overwritten.`)) return;
    setBusy(true);
    try {
      const done = await invoke<Plan>('undo_organization', { plan: run });
      setRuns((previous) => [done, ...previous.slice(1)]);
      setPlan(done);
      setUndoPreview(null);
      setNotice(lang === 'ko' ? '마지막 정리를 되돌렸습니다.' : 'Last organization was undone.');
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function scanDuplicates() {
    if (!path) return;
    setBusy(true);
    try {
      const roots = paths.length ? paths : [path];
      setDuplicates((await Promise.all(roots.map((rootPath) => invoke<Duplicate[]>('find_duplicates', { rootPath, includeSubfolders: sub })))).flat());
      setDuplicateSelection([]);
      setPanel('duplicates');
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function scanLargeFiles() {
    if (!path) return;
    setBusy(true);
    try {
      const roots = paths.length ? paths : [path];
      const threshold = Math.max(1, Number(largeThresholdMb) || 100) * 1024 * 1024;
      setLargeFiles((await Promise.all(roots.map((rootPath) => invoke<LargeFile[]>('find_large_files', { rootPath, includeSubfolders: sub, minimumSizeBytes: threshold })))).flat());
      setCleanupSelection([]);
      setPanel('large');
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function scanEmptyFolders() {
    if (!path) return;
    setBusy(true);
    try {
      const roots = paths.length ? paths : [path];
      setEmptyFolders((await Promise.all(roots.map((rootPath) => invoke<string[]>('find_empty_folders', { rootPath, includeSubfolders: sub })))).flat());
      setCleanupSelection([]);
      setPanel('empty');
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function recycleSelected(pathsToRecycle: string[]) {
    if (!pathsToRecycle.length) return;
    const confirmed = confirm(lang === 'ko' ? `선택한 ${pathsToRecycle.length}개 항목을 휴지통으로 보낼까요?` : `Move ${pathsToRecycle.length} selected item(s) to the Recycle Bin?`);
    if (!confirmed) return;
    setBusy(true);
    try {
      const moved = await invoke<string[]>('recycle_files', { paths: pathsToRecycle });
      setDuplicateSelection((previous) => previous.filter((item) => !moved.includes(item)));
      setCleanupSelection((previous) => previous.filter((item) => !moved.includes(item)));
      setDuplicates((previous) => previous.map((group) => ({ ...group, filePaths: group.filePaths.filter((item) => !moved.includes(item)), reclaimableBytes: group.sizeBytes * Math.max(0, group.filePaths.filter((item) => !moved.includes(item)).length - 1) })).filter((group) => group.filePaths.length > 1));
      setEmptyFolders((previous) => previous.filter((item) => !moved.includes(item)));
      setNotice(lang === 'ko' ? `${moved.length}개 항목을 휴지통으로 보냈습니다.` : `${moved.length} item(s) moved to the Recycle Bin.`);
    } catch (error) {
      setNotice(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function runAiClassification() {
    if (!plan || !selectedProvider || !plan.actions.length) return;
    if (selectedProvider.id === 'ollama' && !selectedProvider.available) {
      setAiMessage(lang === 'ko' ? 'Ollama가 실행 중인지 확인하세요.' : 'Start Ollama before classification.');
      setPanel('ai');
      return;
    }
    const requestTotal = requestCount(plan.actions.length);
    const confirmed = confirm(
      lang === 'ko'
        ? `${selectedProvider.displayName} (${selectedProvider.model})로 ${plan.actions.length}개 파일의 메타데이터를 ${requestTotal}회 요청합니다. 파일 내용과 전체 경로는 보내지 않습니다. 계속할까요?`
        : `Send metadata for ${plan.actions.length} files to ${selectedProvider.displayName} (${selectedProvider.model}) in ${requestTotal} request(s)? File contents and full paths are never sent.`,
    );
    if (!confirmed) return;
    const files = plan.actions.map((action, id) => {
      const name = basename(action.sourcePath);
      const dot = name.lastIndexOf('.');
      return {
        id,
        name,
        extension: dot > 0 ? name.slice(dot + 1).toLowerCase() : '',
        sizeBytes: action.sizeBytes ?? 0,
        modifiedUnixSecs: action.modifiedUnixSecs ?? null,
      };
    });
    const categories = Array.from(new Set([...plan.actions.map((action) => action.category), ...cats.map((category) => category.name)]));
    setBusy(true);
    setPanel('ai');
    setAiMessage('');
    try {
      const result = await invoke<AiResult>('classify_files', { providerId: selectedProvider.id, files, categories });
      setAiResult(result);
      setApprovedAiIds([]);
      setAiMessage(lang === 'ko' ? `${result.suggestions.length}개 제안을 받았습니다. 검토 후 승인하세요.` : `${result.suggestions.length} suggestions received. Review and approve them.`);
    } catch (error) {
      setAiMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function applyAiSuggestions() {
    if (!plan || !aiResult || !approvedAiIds.length) return;
    const approved = aiResult.suggestions.filter((suggestion) => approvedAiIds.includes(suggestion.id));
    setBusy(true);
    try {
      const next = await invoke<Plan>('apply_ai_suggestions', { plan, suggestions: approved });
      const proposed = approved.filter((suggestion) => suggestion.isNewCategory);
      if (proposed.length) {
        setCats((previous) => {
          const existingNames = new Set(previous.map((category) => category.name.toLowerCase()));
          const additions = proposed.filter((suggestion) => !existingNames.has(suggestion.category.toLowerCase())).map((suggestion) => {
            const action = plan.actions[suggestion.id];
            const name = action ? basename(action.sourcePath) : '';
            const dot = name.lastIndexOf('.');
            return { name: suggestion.category, extensions: dot > 0 ? [name.slice(dot + 1).toLowerCase()] : [], enabled: true };
          }).filter((category) => category.extensions.length);
          return [...previous, ...additions];
        });
      }
      setPlan(next);
      setAiResult(null);
      setApprovedAiIds([]);
      setPanel('plan');
      setNotice(lang === 'ko' ? '승인한 AI 제안을 정리 계획에 반영했습니다. 미리보기를 확인하세요.' : 'Approved AI suggestions are in the plan. Review it before moving files.');
    } catch (error) {
      setAiMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function refreshAiProviders() {
    try {
      const providers = await invoke<AiProvider[]>('get_ai_provider_settings');
      setAiProviders(providers);
      const active = providers.find((provider) => provider.id === selectedAi) ?? providers[0];
      if (active) setAiModelDraft(active.model);
    } catch (error) {
      setAiMessage(String(error));
    }
  }

  async function saveAiProvider() {
    const provider = selectedProvider;
    if (!provider) return;
    setBusy(true);
    try {
      await invoke('save_ai_provider', { providerId: provider.id, model: aiModelDraft.trim(), apiKey: aiKeyDraft.trim() || null, enabled: provider.enabled });
      setAiKeyDraft('');
      await refreshAiProviders();
      setAiMessage(lang === 'ko' ? '암호화하여 저장했습니다.' : 'Saved in encrypted storage.');
    } catch (error) {
      setAiMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function testAiProvider() {
    if (!selectedProvider) return;
    setBusy(true);
    try {
      await invoke('test_ai_provider_connection', { providerId: selectedProvider.id });
      setAiMessage(lang === 'ko' ? '연결 테스트에 성공했습니다.' : 'Connection test succeeded.');
    } catch (error) {
      setAiMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function removeAiKey() {
    if (!selectedProvider || selectedProvider.id === 'ollama') return;
    setBusy(true);
    try {
      await invoke('remove_ai_provider_key', { providerId: selectedProvider.id });
      await refreshAiProviders();
      setAiMessage(lang === 'ko' ? '저장된 API 키를 삭제했습니다.' : 'Saved API key removed.');
    } catch (error) {
      setAiMessage(String(error));
    } finally {
      setBusy(false);
    }
  }

  async function checkForUpdate(manual: boolean) {
    if (manual) setBusy(true);
    if (manual) setNotice(t.updateLoading);
    try {
      const available = await check();
      setPendingUpdate(available);
      if (available) {
        setNotice(t.updateAvailable);
        if (shouldOpenUpdateModal(manual, Boolean(available))) setUpdateModal(true);
      } else if (manual) {
        setNotice(t.latest);
      }
    } catch (error) {
      if (manual) setNotice(String(error));
    } finally {
      if (manual) setBusy(false);
    }
  }

  async function installUpdate() {
    if (!pendingUpdate) return;
    setBusy(true);
    setUpdateProgress(t.updateInstalling);
    try {
      await pendingUpdate.downloadAndInstall(undefined, { restartAfterInstall: true });
    } catch (error) {
      setUpdateProgress('');
      setNotice(String(error));
      setBusy(false);
    }
  }

  useEffect(() => {
    void refreshAiProviders();
    void invoke<KnownFolder[]>('known_folders').then(setKnownFolders).catch(() => undefined);
    void invoke<string | null>('startup_path')
      .then((initialPath) => {
        if (shouldUseStartupPath(initialPath)) {
          setPaths([initialPath]);
          setPath(initialPath);
          setPanel('plan');
        }
      })
      .catch((error) => setNotice(String(error)));
    // Startup checks are silent: they only set the update badge/notice.
    void checkForUpdate(false);
  }, []);

  useEffect(() => {
    localStorage.setItem('filemoa-language', lang);
    localStorage.setItem('filemoa-categories', JSON.stringify(cats));
    localStorage.setItem('filemoa-keyword-rules', JSON.stringify(rules));
    localStorage.setItem('filemoa-excluded-paths', JSON.stringify(excludedPaths));
    localStorage.setItem('filemoa-excluded-extensions', JSON.stringify(excludedExtensions));
    localStorage.setItem('filemoa-basis', basis);
    localStorage.setItem('filemoa-mode', mode);
    localStorage.setItem('filemoa-date-basis', dateBasis);
    localStorage.setItem('filemoa-large-threshold-mb', largeThresholdMb);
    localStorage.setItem('filemoa-runs', JSON.stringify(runs));
  }, [lang, cats, rules, excludedPaths, excludedExtensions, basis, mode, dateBasis, largeThresholdMb, runs]);

  useEffect(() => {
    const provider = aiProviders.find((item) => item.id === selectedAi);
    if (provider) setAiModelDraft(provider.model);
  }, [selectedAi, aiProviders]);

  function addCategory() {
    const extensions = exts.split(',').map((value) => value.trim().replace(/^\./, '')).filter(Boolean);
    if (!name.trim() || !extensions.length) return;
    setCats((previous) => [...previous, { name: name.trim(), extensions, enabled: true }]);
    setName('');
    setExts('');
  }

  function removeCategory(index: number) {
    setCats((previous) => previous.filter((_, itemIndex) => itemIndex !== index));
  }

  function addRule() {
    if (!ruleKeyword.trim() || !ruleCategory.trim()) return;
    setRules((previous) => [...previous, { keyword: ruleKeyword.trim(), category: ruleCategory.trim() }]);
    setRuleKeyword('');
    setRuleCategory('');
  }

  function removeRule(index: number) {
    setRules((previous) => previous.filter((_, itemIndex) => itemIndex !== index));
  }

  function addExclusion() {
    const value = excludedDraft.trim();
    if (!value) return;
    if (value.startsWith('.') || /^[a-z0-9]+$/i.test(value)) {
      const extension = value.replace(/^\./, '').toLowerCase();
      setExcludedExtensions((previous) => previous.includes(extension) ? previous : [...previous, extension]);
    } else {
      setExcludedPaths((previous) => previous.includes(value) ? previous : [...previous, value]);
    }
    setExcludedDraft('');
  }

  const ribbon = tab === 'organize'
    ? [
        { key: 'folder', icon: FolderOpen, action: choose },
        { key: 'preview', icon: Search, action: makePlan, disabled: !path },
        { key: 'execute', icon: Play, action: execute, disabled: !plan || !total },
        { key: 'aiClassify', icon: Bot, action: runAiClassification, disabled: !plan || !total },
      ]
    : tab === 'manage'
      ? [
          { key: 'category', icon: Tags, action: () => setPanel('categories') },
          { key: 'duplicates', icon: Copy, action: scanDuplicates, disabled: !path },
          { key: 'large', icon: Search, action: scanLargeFiles, disabled: !path },
          { key: 'emptyFolders', icon: FolderTree, action: scanEmptyFolders, disabled: !path },
          { key: 'aiSettings', icon: Bot, action: () => setPanel('ai') },
        ]
      : [
          { key: 'undo', icon: Undo2, action: () => openUndoPreview(), disabled: !runs.length },
          { key: 'history', icon: History, action: () => setPanel('history') },
        ];

  return (
    <main>
      <header>
        <div className="brand">
          <Files size={29} />
          <div><h1>FileMoa</h1><p>{lang === 'ko' ? '안전하게 정리하고, 언제든 되돌리세요' : 'Organize safely. Undo anytime.'}</p></div>
        </div>
        <button className="settings" onClick={() => setSettings((value) => !value)} aria-label={t.settings}>
          <Settings size={20} />{pendingUpdate && <span className="update-dot" />}
        </button>
        {settings && <div className="settings-menu">
          <button onClick={() => void checkForUpdate(true)} disabled={busy}><RefreshCw size={15} /> {t.update}</button>
          <button onClick={() => setLang((value) => value === 'ko' ? 'en' : 'ko')}><span className="menu-language">A/가</span> {t.language}</button>
        </div>}
      </header>

      <nav className="tabs">
        {(['organize', 'manage', 'safety'] as Tab[]).map((item) => <button className={tab === item ? 'active' : ''} onClick={() => setTab(item)} key={item}>{t[item]}</button>)}
      </nav>

      <section className="ribbon">
        {ribbon.map(({ key, icon: Icon, action, disabled }) => <button key={key} onClick={() => void action()} disabled={busy || disabled}><Icon size={22} /><span>{t[key as keyof typeof t]}</span></button>)}
        {tab === 'organize' && <>
          {knownFolders.length > 0 && <div className="quick-folders" aria-label={t.quickFolders}><span className="quick-folders-label">{t.quickFolders}</span>{knownFolders.map((folder) => <button className="quick-folder" key={folder.id} onClick={() => chooseKnownFolder(folder)} disabled={busy} title={folder.path}><FolderOpen size={15} />{knownFolderLabel(folder.id)}</button>)}</div>}
          <label className="ribbon-option"><input type="checkbox" checked={sub} onChange={(event) => setSub(event.target.checked)} />{t.sub}</label>
          <label className="ribbon-option"><span>{t.basis}</span><select value={basis} onChange={(event) => setBasis(event.target.value)}><option value="type">{t.byType}</option><option value="date">{t.byDate}</option><option value="year">{t.byYear}</option><option value="size">{t.bySize}</option><option value="name">{t.byName}</option></select></label>
          {(basis === 'date' || basis === 'year') && <label className="ribbon-option"><span>{t.dateBasis}</span><select value={dateBasis} onChange={(event) => setDateBasis(event.target.value)}><option value="created">{t.byCreated}</option><option value="modified">{t.byModified}</option><option value="accessed">{t.byAccessed}</option></select></label>}
          <label className="ribbon-option"><span>{t.mode}</span><select value={mode} onChange={(event) => setMode(event.target.value)}><option value="safe">{t.safeMode}</option><option value="normal">{t.normalMode}</option><option value="strong">{t.strongMode}</option></select></label>
          <label className="ribbon-option"><input type="checkbox" checked={includeHidden} onChange={(event) => setIncludeHidden(event.target.checked)} />{t.includeHidden}</label>
          <label className="ribbon-option"><input type="checkbox" checked={multiRoot} onChange={(event) => { setMultiRoot(event.target.checked); if (!event.target.checked && paths.length > 1) { setPaths(paths.slice(0, 1)); setPath(paths[0]); } }} />{t.multiRoot}</label>
        </>}
        <code>{path || t.empty}</code>
      </section>

      {notice && <p className="notice">{notice}</p>}

      <section className="workspace">
        {panel === 'plan' && <>
          <h2>{t.plan}</h2>
          {plan ? <>
            <p>{total} files · {plan.exclusions.length} exclusions · {plan.errors.length} errors{plan.warnings?.length ? ` · ${plan.warnings.length} warnings` : ''}</p>
            {plan.warnings && plan.warnings.length > 0 && <details className="plan-details warning-details" open><summary>{t.warnings} ({plan.warnings.length})</summary>{plan.warnings.map((item, index) => <p key={`${item}-${index}`}>{item}</p>)}</details>}
            <div className="table"><div className="row table-heading"><b>{t.category}</b><span>{lang === 'ko' ? '현재 위치' : 'Current path'}</span><span>{lang === 'ko' ? '이동 위치' : 'Destination'}</span><small>{t.status}</small></div>{plan.actions.map((action, index) => <div className={`row ${action.status !== 'proposed' ? `action-${action.status}` : ''}`} key={`${action.sourcePath}-${index}`}><b>{action.category}</b><span title={action.sourcePath}>{action.sourcePath}</span><span title={action.destinationPath}>→ {action.destinationPath}{action.collision && <em className="collision-badge">{t.collision}</em>}</span><small>{action.reason} · {action.status === 'proposed' ? (lang === 'ko' ? '이동 예정' : 'Pending') : action.status}</small></div>)}</div>
            {plan.exclusions.length > 0 && <details className="plan-details"><summary>{lang === 'ko' ? '제외된 항목' : 'Excluded items'} ({plan.exclusions.length})</summary>{plan.exclusions.map((item, index) => <p key={`${item}-${index}`}>{item}</p>)}</details>}
            {plan.errors.length > 0 && <details className="plan-details error-details" open><summary>{t.errors} ({plan.errors.length})</summary>{plan.errors.map((item, index) => <p key={`${item}-${index}`}>{item}</p>)}</details>}
          </> : <Empty text={t.nothing} />}
        </>}

        {panel === 'categories' && <>
          <h2>{t.catTitle}</h2><p>{t.catHint}</p>
          <div className="form"><input value={name} onChange={(event) => setName(event.target.value)} placeholder={lang === 'ko' ? '카테고리 이름' : 'Category name'} /><input value={exts} onChange={(event) => setExts(event.target.value)} placeholder="pdf,xlsx" /><button onClick={addCategory}>{t.add}</button></div>
          {cats.map((category, index) => <div className="card category-card" key={`${category.name}-${index}`}><b>{category.name}</b><span>.{category.extensions.join(', .')}</span><label><input type="checkbox" checked={category.enabled} onChange={() => setCats((previous) => previous.map((item, itemIndex) => itemIndex === index ? { ...item, enabled: !item.enabled } : item))} />{category.enabled ? 'ON' : 'OFF'}</label><button className="danger-action" onClick={() => removeCategory(index)}><Trash2 size={14} />{lang === 'ko' ? '삭제' : 'Remove'}</button></div>)}
          <h3 className="section-title">{lang === 'ko' ? '파일명 규칙 (IF → THEN)' : 'Filename rules (IF → THEN)'}</h3>
          <p>{lang === 'ko' ? '파일명에 키워드가 포함되면 지정 카테고리를 우선 적용합니다.' : 'A matching keyword takes precedence and routes the file to the chosen category.'}</p>
          <div className="form"><input value={ruleKeyword} onChange={(event) => setRuleKeyword(event.target.value)} placeholder={lang === 'ko' ? '키워드 예: 영수증' : 'Keyword, e.g. receipt'} /><input value={ruleCategory} onChange={(event) => setRuleCategory(event.target.value)} placeholder={lang === 'ko' ? '대상 폴더' : 'Destination category'} /><button onClick={addRule}>{t.add}</button></div>
          {rules.map((rule, index) => <div className="card rule-card" key={`${rule.keyword}-${index}`}><span>IF <b>{rule.keyword}</b> → THEN <b>{rule.category}</b></span><button className="danger-action" onClick={() => removeRule(index)}><Trash2 size={14} />{lang === 'ko' ? '삭제' : 'Remove'}</button></div>)}
          <h3 className="section-title">{t.exceptions}</h3><p>{t.exceptionHint}</p>
          <div className="form"><input value={excludedDraft} onChange={(event) => setExcludedDraft(event.target.value)} placeholder={lang === 'ko' ? '예: node_modules 또는 .env' : 'e.g. node_modules or .env'} /><button onClick={addExclusion}>{t.addException}</button></div>
          {[...excludedPaths.map((value) => ({ value, kind: 'path' })), ...excludedExtensions.map((value) => ({ value: `.${value}`, kind: 'extension' }))].map((item) => <div className="card rule-card" key={`${item.kind}-${item.value}`}><span>{item.kind === 'path' ? '📁' : '✦'} {item.value}</span><button className="danger-action" onClick={() => item.kind === 'path' ? setExcludedPaths((previous) => previous.filter((value) => value !== item.value)) : setExcludedExtensions((previous) => previous.filter((value) => `.${value}` !== item.value))}><Trash2 size={14} />{lang === 'ko' ? '삭제' : 'Remove'}</button></div>)}
        </>}

        {panel === 'duplicates' && <>
          <h2>{t.duplicates}</h2><p>{lang === 'ko' ? '완전히 동일한 파일만 표시합니다. 선택한 파일은 확인 후 휴지통으로 보낼 수 있습니다.' : 'Only byte-identical files are shown. Selected files can be moved to the Recycle Bin after confirmation.'}</p>
          {duplicates.length ? <>{duplicates.map((group, index) => <div className="card" key={`${group.hash}-${index}`}><b>{group.filePaths.length} files · {(group.reclaimableBytes / 1024 / 1024).toFixed(1)} MB</b><span className="muted">SHA-256: {group.hash}</span>{group.filePaths.map((filePath, fileIndex) => <label className="check-row" key={filePath}><input type="checkbox" checked={duplicateSelection.includes(filePath)} onChange={() => setDuplicateSelection((previous) => previous.includes(filePath) ? previous.filter((item) => item !== filePath) : [...previous, filePath])} />{fileIndex === 0 ? (lang === 'ko' ? '보존 권장: ' : 'Keep recommended: ') : ''}{filePath}</label>)}</div>)}<button className="secondary-action" onClick={() => void recycleSelected(duplicateSelection)} disabled={busy || !duplicateSelection.length}><Trash2 size={15} /> {t.recycle} ({duplicateSelection.length})</button></> : <Empty text={t.nothing} />}
        </>}

        {panel === 'large' && <>
          <h2>{t.large}</h2><p>{t.largeHint}</p><label className="threshold-field">{t.largeThreshold}<input type="number" min="1" step="1" value={largeThresholdMb} onChange={(event) => setLargeThresholdMb(event.target.value)} /></label>
          {largeFiles.length ? <div className="card-list">{largeFiles.map((file) => <div className="card compact-card" key={file.path}><b>{formatBytes(file.sizeBytes)}</b><span>{file.path}</span></div>)}</div> : <Empty text={t.nothing} />}
        </>}

        {panel === 'empty' && <>
          <h2>{t.emptyFolders}</h2><p>{t.emptyHint}</p>
          {emptyFolders.length ? <><div className="card-list">{emptyFolders.map((folder) => <label className="check-row card" key={folder}><input type="checkbox" checked={cleanupSelection.includes(folder)} onChange={() => setCleanupSelection((previous) => previous.includes(folder) ? previous.filter((item) => item !== folder) : [...previous, folder])} />{folder}</label>)}</div><button className="secondary-action" onClick={() => void recycleSelected(cleanupSelection)} disabled={busy || !cleanupSelection.length}><Trash2 size={15} /> {t.recycle} ({cleanupSelection.length})</button></> : <Empty text={t.nothing} />}
        </>}

        {panel === 'history' && <>
          <h2>{t.history}</h2>
          {runs.length ? runs.map((run, index) => <div className="card" key={`${run.rootPath}-${index}`}><b>{run.actions.filter((action) => action.status === 'executed').length} files moved</b><span>{run.rootPath}</span><span className="muted">{run.createdAt ? new Date(run.createdAt).toLocaleString() : ''} · {run.exclusions.length} exclusions · {run.errors.length} errors</span>{index === 0 && <button onClick={() => openUndoPreview(run)}>{t.undo}</button>}</div>) : <Empty text={t.nothing} />}
        </>}

        {panel === 'ai' && <>
          <div className="panel-heading"><div><h2>{t.aiTitle}</h2><p>{t.aiHint}</p></div><button className="icon-button" onClick={() => void refreshAiProviders()} disabled={busy} aria-label={t.retry}><RefreshCw size={18} /></button></div>
          <div className="privacy-callout"><ShieldCheck size={19} /><span>{t.localOnly}</span></div>
          <h3>{t.selected}</h3>
          <div className="provider-grid">{aiProviders.map((provider) => <button className={`provider-card ${selectedAi === provider.id ? 'selected' : ''}`} key={provider.id} onClick={() => { setSelectedAi(provider.id); setAiKeyDraft(''); }}><span className="provider-name"><Bot size={17} /> {provider.displayName}</span><span className="provider-state">{providerStatus(provider.id, provider.keyPresent, provider.available, lang)}</span></button>)}</div>
          {selectedProvider && <div className="ai-editor"><div className="ai-editor-title"><h3>{selectedProvider.displayName}</h3>{selectedProvider.id === 'ollama' && <span className={selectedProvider.available ? 'status-good' : 'status-muted'}>{selectedProvider.endpoint}</span>}</div>
            {selectedProvider.id !== 'ollama' && <label className="field-label">{t.apiKey}<input type="password" value={aiKeyDraft} onChange={(event) => setAiKeyDraft(event.target.value)} placeholder={selectedProvider.keyPresent ? '••••••••••••••••' : 'sk-…'} autoComplete="new-password" /></label>}
            <label className="field-label">{t.model}{selectedProvider.id === 'ollama' && selectedProvider.ollamaModels.length ? <select value={aiModelDraft} onChange={(event) => setAiModelDraft(event.target.value)}>{selectedProvider.ollamaModels.map((model) => <option value={model} key={model}>{model}</option>)}</select> : <input value={aiModelDraft} onChange={(event) => setAiModelDraft(event.target.value)} placeholder={selectedProvider.model} />}<small>{t.modelHint}</small></label>
            <div className="ai-actions"><button onClick={() => void saveAiProvider()} disabled={busy || (selectedProvider.id !== 'ollama' && !aiKeyDraft && !selectedProvider.keyPresent)}><CheckCircle2 size={16} /> {t.save}</button><button onClick={() => void testAiProvider()} disabled={busy || (selectedProvider.id !== 'ollama' && !selectedProvider.keyPresent && !aiKeyDraft)}><RefreshCw size={16} /> {t.test}</button>{selectedProvider.id !== 'ollama' && selectedProvider.keyPresent && <button className="danger-action" onClick={() => void removeAiKey()} disabled={busy}><Trash2 size={16} /> {t.removeKey}</button>}</div>
            {selectedProvider.id !== 'ollama' && <a className="registration-link" href={selectedProvider.registrationUrl} target="_blank" rel="noreferrer"><ExternalLink size={15} /> {t.getKey}</a>}
            {selectedProvider.id === 'ollama' && <p className="ollama-help"><Clock3 size={15} /> {selectedProvider.ollamaModels.length ? `${t.ollamaModels}: ${selectedProvider.ollamaModels.join(', ')}` : t.ollamaMissing}</p>}
            {aiMessage && <p className="inline-message">{aiMessage}</p>}
          </div>}
          {aiResult && <div className="ai-results">
            <div className="ai-results-heading"><h3>{t.aiResults}</h3><span className="muted">{t.requestCount}: {aiResult.requestCount}</span></div>
            {aiResult.proposedCategories.length > 0 && <p className="new-category-note">{t.newCategory}: {aiResult.proposedCategories.join(', ')}</p>}
            <div className="suggestion-list">{aiResult.suggestions.map((suggestion) => <div className={`suggestion ${suggestion.needsReview ? 'needs-review' : ''}`} key={suggestion.id}>
              <input type="checkbox" checked={approvedAiIds.includes(suggestion.id)} onChange={() => setApprovedAiIds((previous) => previous.includes(suggestion.id) ? previous.filter((id) => id !== suggestion.id) : [...previous, suggestion.id])} />
              <div><b>{plan?.actions[suggestion.id] ? basename(plan.actions[suggestion.id].sourcePath) : `#${suggestion.id}`}</b><span className="suggestion-category">→ {suggestion.category} · {(suggestion.confidence * 100).toFixed(0)}%</span><small>{suggestion.needsReview ? `${t.reviewRequired}: ` : ''}{suggestion.reason}{suggestion.isNewCategory ? ` · ${t.newCategory}` : ''}</small></div>
            </div>)}</div>
            <button className="primary-action" onClick={() => void applyAiSuggestions()} disabled={busy || !approvedAiIds.length}><CheckCircle2 size={16} /> {t.applySuggestions}</button>
          </div>}
        </>}
      </section>

      {undoPreview && <div className="modal-backdrop" role="presentation"><div className="update-modal undo-modal" role="dialog" aria-modal="true" aria-labelledby="undo-title"><button className="modal-close" onClick={() => setUndoPreview(null)} disabled={busy} aria-label={t.cancel}><X size={18} /></button><Undo2 size={30} className="modal-icon" /><h2 id="undo-title">{t.undoPreview}</h2><p>{lang === 'ko' ? '실행된 파일만 원래 위치로 복원합니다. 원래 위치에 새 파일이 있으면 덮어쓰지 않고 충돌로 기록합니다.' : 'Only files from this run are restored. A newer file at the original path is never overwritten and is reported as a conflict.'}</p><div className="release-notes undo-list">{undoPreview.actions.filter((action) => action.status === 'executed').slice(0, 100).map((action, index) => <div key={`${action.sourcePath}-${index}`}>{action.destinationPath} → {action.sourcePath}</div>)}</div><div className="modal-actions"><button className="primary-action" onClick={() => void undo(undoPreview)} disabled={busy}><Undo2 size={16} /> {t.undoConfirm}</button><button className="secondary-action" onClick={() => setUndoPreview(null)} disabled={busy}><X size={16} /> {t.cancel}</button></div></div></div>}

      {updateModal && pendingUpdate && <div className="modal-backdrop" role="presentation"><div className="update-modal" role="dialog" aria-modal="true" aria-labelledby="update-title"><button className="modal-close" onClick={() => setUpdateModal(false)} disabled={busy} aria-label={t.installLater}><X size={18} /></button><Download size={30} className="modal-icon" /><h2 id="update-title">{t.updateTitle}</h2><div className="version-line"><span>{t.currentVersion}: {pendingUpdate.currentVersion}</span><span>→</span><strong>{t.newVersion}: {pendingUpdate.version}</strong></div>{pendingUpdate.date && <p className="muted">{pendingUpdate.date}</p>}<div className="release-notes">{pendingUpdate.body || (lang === 'ko' ? '릴리즈 노트가 제공되지 않았습니다.' : 'No release notes were provided.')}</div>{updateProgress && <p className="update-progress"><RefreshCw size={15} /> {updateProgress}</p>}<div className="modal-actions"><button className="primary-action" onClick={() => void installUpdate()} disabled={busy}><Download size={16} /> {t.installNow}</button><button className="secondary-action" onClick={() => setUpdateModal(false)} disabled={busy}><Clock3 size={16} /> {t.installLater}</button></div></div></div>}
    </main>
  );
}

function Empty({ text }: { text: string }) {
  return <div className="empty"><FolderTree size={36} /><p>{text}</p></div>;
}
