/** Smallest positive integer not present in `usedNumbers`. Used to name
 * auto-numbered "Local Shell N" tabs so that closing tabs frees their slot
 * instead of the counter climbing forever (open 5, close all, reopen one
 * should be "Local Shell 1", not "Local Shell 6"). */
export function nextAvailableNumber(usedNumbers: Iterable<number>): number {
  const used = new Set(usedNumbers);
  let n = 1;
  while (used.has(n)) n++;
  return n;
}
