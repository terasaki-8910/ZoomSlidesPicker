/** True when running on macOS (the Cmd/Ctrl+, split, and shortcut notation). */
export function isMac(): boolean {
  if (typeof navigator === "undefined") return false;
  const uaData = (navigator as Navigator & { userAgentData?: { platform?: string } }).userAgentData;
  const platform = uaData?.platform || navigator.platform || navigator.userAgent || "";
  return /mac/i.test(platform);
}
