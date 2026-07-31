import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { CompanionSkin, VisualPreset } from '../../lib/companion';
import { CompanionOrb } from './CompanionOrb';

describe('CompanionOrb', () => {
  it.each([
    ['quiet_aurora', 'companion-orb--aurora'],
    ['porcelain_pearl', 'companion-orb--pearl'],
    ['deep_ink', 'companion-orb--ink'],
  ] as const)('renders the complete %s preset', (preset, className) => {
    const { container } = render(
      <CompanionOrb skin={skinWith(preset)} motionEnabled status="idle" />,
    );

    expect(screen.getByTestId('companion-orb')).toHaveClass(className);
    expect(container.querySelectorAll('.companion-orb__ring-runner')).toHaveLength(2);
    expect(container.querySelectorAll('.companion-orb__surface-current')).toHaveLength(1);
    expect(container.querySelectorAll('.companion-orb__star-point')).toHaveLength(5);
    expect(container.querySelectorAll('.companion-orb__central-star')).toHaveLength(1);
  });

  it('turns off all authored motion when motion is disabled', () => {
    render(<CompanionOrb skin={skinWith('quiet_aurora')} motionEnabled={false} status="idle" />);

    expect(screen.getByTestId('companion-orb')).toHaveClass('companion-orb--motion-off');
  });
});

function skinWith(visualPreset: VisualPreset): CompanionSkin {
  return {
    id: visualPreset,
    name: visualPreset,
    source: visualPreset === 'custom' ? 'image' : 'builtin',
    visualPreset,
    texturePath: null,
    previewPath: null,
    flowColors: ['#BD9FFF', '#FFF4DC'],
    flowSpeed: 1,
    flowIntensity: 0.7,
    createdAt: '2026-07-31T00:00:00Z',
  };
}
