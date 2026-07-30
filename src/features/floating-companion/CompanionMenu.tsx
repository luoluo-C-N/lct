type CompanionMenuProps = {
  onCapture: (mode: 'region' | 'window' | 'fullscreen') => void;
  onImport: () => void;
};

export function CompanionMenu({ onCapture, onImport }: CompanionMenuProps) {
  return (
    <menu aria-label="悬浮角色菜单">
      <button type="button" onClick={() => onCapture('region')}>区域截图</button>
      <button type="button" onClick={() => onCapture('window')}>窗口截图</button>
      <button type="button" onClick={() => onCapture('fullscreen')}>全屏截图</button>
      <button type="button" onClick={onImport}>导入图片</button>
    </menu>
  );
}
