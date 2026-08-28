import { describe, expect, it } from 'vitest';
import { nextCue, scoreSample, type PlayerRound } from './game';

const player = (): PlayerRound => ({ id: 'p1', score: 0, hitCue: false, lastShakeAt: 0 });

describe('game rules', () => {
  it('alternates freeze cues', () => expect(nextCue('freeze', 'MOVE')).toBe('FREEZE'));
  it('does not repeat a tilt direction', () => expect(nextCue('tilt', 'LEFT', () => 0)).not.toBe('LEFT'));
  it('scores a correct tilt only once per cue', () => {
    const p = player(); p.sample = { x: 24, y: 0, shake: 0, at: 1 };
    expect(scoreSample('tilt', 'RIGHT', p, 100)).toBe(10);
    expect(scoreSample('tilt', 'RIGHT', p, 200)).toBe(0);
  });
  it('rewards stillness during freeze', () => {
    const p = player(); p.prior = { x: 1, y: 1, shake: 0, at: 1 }; p.sample = { x: 1.2, y: 1.1, shake: 0.2, at: 2 };
    expect(scoreSample('freeze', 'FREEZE', p, 100)).toBe(1);
  });
  it('debounces shake scoring', () => {
    const p = player(); p.sample = { x: 0, y: 0, shake: 9, at: 1 };
    expect(scoreSample('shake', 'GO', p, 500)).toBe(2);
    expect(scoreSample('shake', 'GO', p, 600)).toBe(0);
  });
});
