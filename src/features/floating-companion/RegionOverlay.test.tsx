import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { RegionOverlay } from './RegionOverlay';

it('normalizes a reverse drag and converts it to physical pixels', () => {
  const onSelect = vi.fn();
  render(
    <RegionOverlay
      scaleFactor={1.5}
      previewDataUrl="data:image/png;base64,fixture"
      onSelect={onSelect}
      onCancel={vi.fn()}
    />,
  );
  const overlay = screen.getByRole('dialog', { name: '选择截图区域' });
  expect(screen.getByRole('img', { name: '截图冻结画面' })).toHaveAttribute(
    'src',
    'data:image/png;base64,fixture',
  );

  fireEvent(overlay, pointerEvent('pointerdown', { button: 0, clientX: 100, clientY: 80 }));
  fireEvent(overlay, pointerEvent('pointermove', { buttons: 1, clientX: 20, clientY: 30 }));
  fireEvent(overlay, pointerEvent('pointerup', { button: 0, clientX: 20, clientY: 30 }));

  expect(onSelect).toHaveBeenCalledWith({ x: 30, y: 45, width: 120, height: 75 });
});

it('cancels on Escape and ignores a zero-area click', async () => {
  const onSelect = vi.fn();
  const onCancel = vi.fn();
  render(
    <RegionOverlay
      scaleFactor={1}
      previewDataUrl="data:image/png;base64,fixture"
      onSelect={onSelect}
      onCancel={onCancel}
    />,
  );
  const overlay = screen.getByRole('dialog', { name: '选择截图区域' });

  fireEvent(overlay, pointerEvent('pointerdown', { button: 0, clientX: 40, clientY: 40 }));
  fireEvent(overlay, pointerEvent('pointerup', { button: 0, clientX: 40, clientY: 40 }));
  expect(onSelect).not.toHaveBeenCalled();

  await userEvent.keyboard('{Escape}');
  expect(onCancel).toHaveBeenCalledTimes(1);
});

function pointerEvent(type: string, properties: Record<string, number>) {
  const event = new Event(type, { bubbles: true });
  Object.entries(properties).forEach(([name, value]) => {
    Object.defineProperty(event, name, { value });
  });
  return event;
}
