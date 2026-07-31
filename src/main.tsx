import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import Root from './Root';
import { markWindowDocument } from './lib/windowDocument';

const windowLabel = getCurrentWebviewWindow().label;
markWindowDocument(windowLabel);

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root windowLabel={windowLabel} />
  </StrictMode>
);
