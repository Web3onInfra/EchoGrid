export interface AppConfig {
  oracleUrl: string;
}

export function getAppEnv(): AppConfig {
  // Check runtime query param or localStorage override for on-device testing
  if (typeof window !== 'undefined') {
    try {
      const urlParams = new URLSearchParams(window.location.search);
      const queryOverride = urlParams.get('oracle');
      if (queryOverride) {
        window.localStorage.setItem('echogrid_oracle_url', queryOverride);
        return { oracleUrl: queryOverride.replace(/\/+$/, '') };
      }
      const storedOverride = window.localStorage.getItem('echogrid_oracle_url');
      if (storedOverride) {
        return { oracleUrl: storedOverride.replace(/\/+$/, '') };
      }
    } catch {
      // Ignore storage/search errors in constrained environments
    }
  }

  const envUrl = import.meta.env.VITE_ORACLE_URL;
  if (envUrl && envUrl.trim().length > 0) {
    return {
      oracleUrl: envUrl.replace(/\/+$/, ''),
    };
  }

  // In production builds (Vercel / live PWA), default to deployed Fly.io oracle
  // to prevent mobile clients from hitting loopback (localhost:8080)
  if (import.meta.env.PROD) {
    return {
      oracleUrl: 'https://echogrid-oracle.fly.dev',
    };
  }

  return {
    oracleUrl: '',
  };
}

export const env = getAppEnv();
