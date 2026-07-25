import { useState } from 'react';
import { CompanionMenu } from './CompanionMenu';
import { capture, type CaptureMode } from '../../lib/desktop';

export function CompanionWindow() {
  const [open, setOpen] = useState(false);
  const [lastAction, setLastAction] = useState('');
  async function startCapture(mode: CaptureMode) {
    setLastAction(`${mode} 截图中`);
    await capture(mode);
    setLastAction(`${mode} 截图已保存`);
  }
  return <aside className="companion" aria-label="悬浮角色"><button type="button" aria-label="悬浮角色" onClick={() => setOpen(!open)}>✦</button>{open && <CompanionMenu onCapture={startCapture} />}{lastAction && <output>{lastAction}</output>}</aside>;
}
