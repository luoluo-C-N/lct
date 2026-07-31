import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import Root from './Root';

const windowLabel = getCurrentWebviewWindow().label;

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root windowLabel={windowLabel} />
  </StrictMode>
);
