import React, { createContext, useContext, useEffect, useState, useCallback } from 'react';
import { PublicConfigResponse, SelfCheckReport } from '../api/types';
import { oracleClient } from '../api/oracleClient';
import { OracleError } from '../api/errors';

export type DegradedReason = 'oracle_down' | 'contract_paused' | 'wrong_network' | null;

export interface OracleState {
  config: PublicConfigResponse | null;
  ready: SelfCheckReport | null;
  loading: boolean;
  error: OracleError | null;
  isDegraded: boolean;
  degradedReason: DegradedReason;
  refresh: () => Promise<void>;
}

const OracleContext = createContext<OracleState>({
  config: null,
  ready: null,
  loading: true,
  error: null,
  isDegraded: false,
  degradedReason: null,
  refresh: async () => {},
});

export const OracleProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [config, setConfig] = useState<PublicConfigResponse | null>(null);
  const [ready, setReady] = useState<SelfCheckReport | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<OracleError | null>(null);

  const fetchBootstrap = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [fetchedConfig, fetchedReady] = await Promise.all([
        oracleClient.getConfig(),
        oracleClient.getReady(),
      ]);

      setConfig(fetchedConfig);
      setReady(fetchedReady);
    } catch (err: unknown) {
      if (err instanceof OracleError) {
        setError(err);
      } else {
        setError(
          new OracleError({
            code: 'ORACLE_UNREACHABLE',
            message: 'Failed to bootstrap Oracle connection',
            status: 503,
          }),
        );
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchBootstrap();
  }, [fetchBootstrap]);

  let degradedReason: DegradedReason = null;
  if (error || !config || !ready) {
    degradedReason = 'oracle_down';
  } else if (config.network !== 'testnet') {
    degradedReason = 'wrong_network';
  } else if (ready.contract_paused) {
    degradedReason = 'contract_paused';
  }

  const isDegraded = degradedReason !== null;

  return (
    <OracleContext.Provider
      value={{
        config,
        ready,
        loading,
        error,
        isDegraded,
        degradedReason,
        refresh: fetchBootstrap,
      }}
    >
      {children}
    </OracleContext.Provider>
  );
};

// eslint-disable-next-line react-refresh/only-export-components
export const useOracle = () => useContext(OracleContext);
