const assetPrefix = '/assets/';

export type TrustedTypes = {
  createPolicy: (name: string, rules: { createScriptURL: (url: string) => string | null }) => unknown;
};

// Whether a script URL may be minted by the one Trusted Types policy (SEC-API-045): same origin, under
// the bundle's asset path. Relative URLs are resolved against the page origin. Anything else is refused.

export function acceptsScriptUrl(url: string, origin: string): boolean {
  let parsed: URL;
  try {
    parsed = new URL(url, origin);
  } catch {
    return false;
  }
  return parsed.origin === origin && parsed.pathname.startsWith(assetPrefix);
}

export function scriptUrl(url: string, origin: string): string | null {
  if (acceptsScriptUrl(url, origin)) {
    return url;
  }
  return null;
}

export function installGunmetalLoader(
  trustedTypes: TrustedTypes | undefined,
  createScriptURL: (url: string) => string | null,
): void {
  if (trustedTypes === undefined) {
    return;
  }
  trustedTypes.createPolicy('gunmetal-loader', { createScriptURL });
}
