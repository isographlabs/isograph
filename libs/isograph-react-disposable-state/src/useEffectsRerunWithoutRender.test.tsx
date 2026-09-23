import { render } from '@testing-library/react';
import React, { useEffect } from 'react';
import { describe, expect, test } from 'vitest';
import { useEffectsRerunWithoutRenderRef } from './useEffectsRerunWithoutRender';

function Probe({ dep, reports }: { dep: number; reports: boolean[] }) {
  const effectsRerunWithoutRenderRef = useEffectsRerunWithoutRenderRef();
  useEffect(() => {
    reports.push(effectsRerunWithoutRenderRef.current);
    effectsRerunWithoutRenderRef.current = false;
  }, [dep, effectsRerunWithoutRenderRef]);
  return null;
}

describe('useEffectsRerunWithoutRenderRef', () => {
  test('under StrictMode, the first mount reads false and the simulated remount reads true', () => {
    const reports: boolean[] = [];
    render(<Probe dep={0} reports={reports} />, { reactStrictMode: true });

    expect(reports).toEqual([false, true]);
  });

  test('without StrictMode, the mount reads false', () => {
    const reports: boolean[] = [];
    render(<Probe dep={0} reports={reports} />, { reactStrictMode: false });

    expect(reports).toEqual([false]);
  });

  test('an effect run caused by a render reads false, once the remount has reset the ref', () => {
    const reports: boolean[] = [];
    const { rerender } = render(<Probe dep={0} reports={reports} />, {
      reactStrictMode: true,
    });
    rerender(<Probe dep={1} reports={reports} />);

    expect(reports).toEqual([false, true, false]);
  });

  test('the ref stays true until the caller resets it', () => {
    const reads: boolean[] = [];
    function NeverResets() {
      const effectsRerunWithoutRenderRef = useEffectsRerunWithoutRenderRef();
      useEffect(() => {
        reads.push(effectsRerunWithoutRenderRef.current);
      });
      return null;
    }
    const { rerender } = render(<NeverResets />, { reactStrictMode: true });
    rerender(<NeverResets />);

    expect(reads).toEqual([false, true, true]);
  });
});
