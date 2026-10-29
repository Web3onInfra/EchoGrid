export interface AppConfig {
  oracleUrl: string;
}

export function getAppEnv(): AppConfig {
  const oracleUrl = import.meta.env.VITE_ORACLE_URL || 'http://localhost:8080';
  return {
    oracleUrl: oracleUrl.replace(/\/+$/, ''),
  };
}

export const env = getAppEnv();
