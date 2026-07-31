import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import Root from './Root';

vi.mock('./App', () => ({
  default: () => <main aria-label="影像资料库" />,
}));

vi.mock('./features/floating-companion/CompanionWindow', () => ({
  CompanionWindow: () => <aside aria-label="悬浮助手" />,
}));

it('renders only the companion entry for the companion window', () => {
  render(<Root windowLabel="companion" />);

  expect(screen.getByRole('complementary', { name: '悬浮助手' })).toBeVisible();
  expect(screen.queryByRole('main', { name: '影像资料库' })).not.toBeInTheDocument();
});

it('renders only the library entry for the main window', () => {
  render(<Root windowLabel="main" />);

  expect(screen.getByRole('main', { name: '影像资料库' })).toBeVisible();
  expect(screen.queryByRole('complementary', { name: '悬浮助手' })).not.toBeInTheDocument();
});
