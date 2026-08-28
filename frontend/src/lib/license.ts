const slug = 'pairplay-motion';
const key = `sb_license:${slug}`;
const verdictKey = `${key}:verdict`;
export const billingBase = (import.meta.env.VITE_BILLING_BASE || 'https://api.sociobot.in').replace(/\/$/, '');
export const checkoutUrl = `${billingBase}/api/v1/products/${slug}/checkout`;

type Verdict = { valid: boolean; checkedAt: number };

export function captureLicense(): string | null {
  const url = new URL(location.href);
  const incoming = url.searchParams.get('license');
  if (incoming) {
    localStorage.setItem(key, incoming);
    localStorage.removeItem(verdictKey);
    url.searchParams.delete('license');
    history.replaceState({}, '', `${url.pathname}${url.search}${url.hash}`);
  }
  return incoming || localStorage.getItem(key);
}

export function cachedUnlock(): boolean {
  try {
    const verdict = JSON.parse(localStorage.getItem(verdictKey) || 'null') as Verdict | null;
    return Boolean(localStorage.getItem(key) && verdict?.valid);
  } catch { return false; }
}

export async function verifyLicense(force = false): Promise<{ unlocked: boolean; notice: string }> {
  const token = localStorage.getItem(key);
  if (!token) return { unlocked: false, notice: '' };
  try {
    const cached = JSON.parse(localStorage.getItem(verdictKey) || 'null') as Verdict | null;
    if (!force && cached && Date.now() - cached.checkedAt < 86_400_000) {
      return { unlocked: cached.valid, notice: cached.valid ? '' : 'License no longer active.' };
    }
    const response = await fetch(`${billingBase}/api/v1/products/${slug}/verify?license=${encodeURIComponent(token)}`);
    if (!response.ok) throw new Error('Verification unavailable');
    const body = await response.json() as { valid: boolean };
    localStorage.setItem(verdictKey, JSON.stringify({ valid: body.valid, checkedAt: Date.now() }));
    return { unlocked: body.valid, notice: body.valid ? '' : 'License no longer active.' };
  } catch {
    return { unlocked: cachedUnlock(), notice: 'Could not refresh the license. Using the last saved result.' };
  }
}

export function saveLicense(token: string): void {
  localStorage.setItem(key, token.trim());
  localStorage.removeItem(verdictKey);
}
