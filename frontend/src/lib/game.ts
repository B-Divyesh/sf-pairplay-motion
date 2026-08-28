export type GameId = 'freeze' | 'tilt' | 'shake';
export type Cue = 'MOVE' | 'FREEZE' | 'LEFT' | 'RIGHT' | 'UP' | 'DOWN' | 'GO' | 'REST';

export interface MotionSample {
  x: number;
  y: number;
  shake: number;
  at: number;
}

export interface PlayerRound {
  id: string;
  score: number;
  hitCue: boolean;
  lastShakeAt: number;
  sample?: MotionSample;
  prior?: MotionSample;
}

export const gameDetails: Record<GameId, { name: string; kicker: string; description: string; paid: boolean }> = {
  freeze: { name: 'Dead Still', kicker: 'The stop-press game', description: 'Move on MOVE. Lock the phone still on FREEZE.', paid: false },
  tilt: { name: 'News Desk', kicker: 'Follow the headline', description: 'Tilt in the printed direction before the edition closes.', paid: true },
  shake: { name: 'Ink Runner', kicker: 'Race the presses', description: 'Shake hard on GO. Hold the presses on REST.', paid: true },
};

export const initialCue = (game: GameId): Cue => game === 'freeze' ? 'MOVE' : game === 'tilt' ? 'LEFT' : 'REST';

export function nextCue(game: GameId, previous: Cue, random = Math.random): Cue {
  if (game === 'freeze') return previous === 'MOVE' ? 'FREEZE' : 'MOVE';
  if (game === 'shake') return previous === 'GO' ? 'REST' : 'GO';
  const choices: Cue[] = ['LEFT', 'RIGHT', 'UP', 'DOWN'];
  const available = choices.filter((cue) => cue !== previous);
  return available[Math.floor(random() * available.length)];
}

export function scoreSample(game: GameId, cue: Cue, player: PlayerRound, now: number): number {
  const sample = player.sample;
  if (!sample) return 0;
  if (game === 'freeze') {
    if (cue !== 'FREEZE' || !player.prior) return 0;
    const drift = Math.abs(sample.x - player.prior.x) + Math.abs(sample.y - player.prior.y) + sample.shake * 0.45;
    return drift < 3.5 ? 1 : 0;
  }
  if (game === 'tilt') {
    if (player.hitCue) return 0;
    const matches = (cue === 'LEFT' && sample.x < -16) || (cue === 'RIGHT' && sample.x > 16)
      || (cue === 'UP' && sample.y < -16) || (cue === 'DOWN' && sample.y > 16);
    if (matches) { player.hitCue = true; return 10; }
    return 0;
  }
  if (cue === 'GO' && sample.shake > 7 && now - player.lastShakeAt > 260) {
    player.lastShakeAt = now;
    return 2;
  }
  return 0;
}

export function cueEvery(game: GameId): number {
  return game === 'tilt' ? 3500 : game === 'freeze' ? 3200 : 2800;
}
