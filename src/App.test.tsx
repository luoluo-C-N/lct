import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import App from './App';

it('renders the library shell', () => {
  render(<App />);

  expect(screen.getByRole('main', { name: '影像资料库' })).toBeVisible();
});
