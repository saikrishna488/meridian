/** Normalize wheel units and retain fractional touchpad motion between events. */
export function wheelScroller() {
  let remainder = 0;
  return (delta: number, mode: number, rowHeight: number, rows: number): number => {
    if (!Number.isFinite(delta) || !delta) return 0;
    const lines = mode === 1 ? delta : mode === 2 ? delta * rows : delta / Math.max(1, rowHeight);
    if (Math.sign(lines) !== Math.sign(remainder)) remainder = 0;
    remainder += lines;
    const whole = Math.trunc(remainder);
    remainder -= whole;
    return whole;
  };
}
