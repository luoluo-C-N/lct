import { useState } from 'react';
import { CompanionMenu } from './CompanionMenu';
import {
  capture,
  importFiles,
  selectImageFiles,
  type CaptureMode,
} from '../../lib/desktop';

export function CompanionWindow() {
  const [open, setOpen] = useState(false);
  const [lastAction, setLastAction] = useState('');
  const [error, setError] = useState('');

  async function startCapture(mode: CaptureMode) {
    setError('');
    setLastAction(`${mode} 截图中`);
    try {
      await capture(mode);
      setLastAction(`${mode} 截图已保存`);
    } catch {
      setLastAction('');
      setError('截图失败，请重试。');
    }
  }

  async function startImport() {
    setError('');
    try {
      const paths = await selectImageFiles();
      if (paths.length === 0) {
        setLastAction('未选择图片');
        return;
      }

      setLastAction('正在导入图片…');
      const assets = await importFiles(paths);
      setLastAction(`已导入 ${assets.length} 张图片`);
    } catch {
      setLastAction('');
      setError('导入失败，请重新选择图片。');
    }
  }

  return (
    <aside className="companion" aria-label="悬浮助手">
      <button type="button" aria-label="悬浮角色" onClick={() => setOpen(!open)}>✦</button>
      {open && <CompanionMenu onCapture={startCapture} onImport={startImport} />}
      {lastAction && <output aria-live="polite">{lastAction}</output>}
      {error && <p role="alert">{error}</p>}
    </aside>
  );
}
