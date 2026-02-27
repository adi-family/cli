import type { InterpolateOptions, SpringConfig } from './types.js';

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

export function interpolate(
  input: number,
  inputRange: readonly number[],
  outputRange: readonly number[],
  options?: InterpolateOptions,
): number {
  if (inputRange.length !== outputRange.length) {
    throw new Error('inputRange and outputRange must have the same length');
  }
  if (inputRange.length < 2) {
    throw new Error('ranges must have at least 2 values');
  }

  const extrapolateLeft = options?.extrapolateLeft ?? 'clamp';
  const extrapolateRight = options?.extrapolateRight ?? 'clamp';

  let segmentIndex = inputRange.findIndex((v, i) => i > 0 && input <= v) - 1;
  if (segmentIndex < 0) segmentIndex = inputRange.length - 2;
  segmentIndex = clamp(segmentIndex, 0, inputRange.length - 2);

  const inMin = inputRange[segmentIndex]!;
  const inMax = inputRange[segmentIndex + 1]!;
  const outMin = outputRange[segmentIndex]!;
  const outMax = outputRange[segmentIndex + 1]!;

  let t = (input - inMin) / (inMax - inMin);

  if (extrapolateLeft === 'clamp' && input < inputRange[0]!) {
    t = 0;
  }
  if (extrapolateRight === 'clamp' && input > inputRange[inputRange.length - 1]!) {
    t = 1;
  }

  return outMin + t * (outMax - outMin);
}

export function spring(params: {
  frame: number;
  fps: number;
  config?: SpringConfig;
}): number {
  const { frame, fps, config } = params;
  const damping = config?.damping ?? 10;
  const mass = config?.mass ?? 1;
  const stiffness = config?.stiffness ?? 100;
  const overshootClamping = config?.overshootClamping ?? false;

  const omega = Math.sqrt(stiffness / mass);
  const zeta = damping / (2 * Math.sqrt(stiffness * mass));
  const t = frame / fps;

  let value: number;
  if (zeta < 1) {
    const omegaD = omega * Math.sqrt(1 - zeta * zeta);
    value = 1 - Math.exp(-zeta * omega * t) *
      (Math.cos(omegaD * t) + (zeta * omega / omegaD) * Math.sin(omegaD * t));
  } else {
    const r1 = -omega * (zeta + Math.sqrt(zeta * zeta - 1));
    const r2 = -omega * (zeta - Math.sqrt(zeta * zeta - 1));
    const c2 = r1 / (r1 - r2);
    const c1 = 1 - c2;
    value = 1 - c1 * Math.exp(r1 * t) - c2 * Math.exp(r2 * t);
  }

  if (overshootClamping) {
    value = clamp(value, 0, 1);
  }

  return value;
}
