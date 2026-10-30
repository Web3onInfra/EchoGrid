export function runCheck(name: string, fn: () => Promise<unknown>): Promise<boolean>;
export function fetchJson<T = unknown>(url: string): Promise<T>;
export function checkOracleHealth(oracleUrl: string): Promise<void>;
export function checkOracleCors(oracleUrl: string, targetOrigin?: string): Promise<void>;
export function checkOracleReady(oracleUrl: string): Promise<void>;
export function checkOracleConfig(oracleUrl: string): Promise<void>;
export function checkOracleChallenge(oracleUrl: string): Promise<void>;
export function checkPwaIndex(targetOrigin: string): Promise<void>;
export function checkPwaManifest(targetOrigin: string): Promise<void>;
export function checkPwaIcons(targetOrigin: string): Promise<void>;
export function checkPwaServiceWorker(targetOrigin: string): Promise<void>;
export function checkPwaHeaders(targetOrigin: string): Promise<void>;

export interface SmokeTestOptions {
  targetOrigin?: string;
  oracleUrl?: string;
  skipOracle?: boolean;
  skipPwa?: boolean;
}

export interface SmokeTestResult {
  passed: number;
  total: number;
  success: boolean;
}

export function runSmokeTests(options?: SmokeTestOptions): Promise<SmokeTestResult>;
