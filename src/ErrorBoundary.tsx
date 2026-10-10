import { Component, type ErrorInfo, type ReactNode } from 'react';

type State = { failed: boolean; message: string };

/** Keeps a render error from blanking the whole window. */
export default class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { failed: false, message: '' };

  static getDerivedStateFromError(error: unknown): State {
    return { failed: true, message: error instanceof Error ? error.message : String(error) };
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    console.error('FileMoa render error', error, info.componentStack);
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div role="alert" style={{ padding: 32, fontFamily: 'sans-serif' }}>
        <h2>화면을 표시하는 중 오류가 발생했습니다 / Something went wrong</h2>
        <p>파일은 변경되지 않았습니다. 다시 불러오면 정리 기록은 그대로 유지됩니다. / Your files were not changed, and your run history is kept.</p>
        <pre style={{ whiteSpace: 'pre-wrap' }}>{this.state.message}</pre>
        <button onClick={() => window.location.reload()}>다시 불러오기 / Reload</button>
      </div>
    );
  }
}
