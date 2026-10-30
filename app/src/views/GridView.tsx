import React, { useState } from 'react';
import { GRID_SCENARIOS, GridScenarioId } from '../grid/scenarios';
import { useOracle } from '../context/OracleContext';
import { Zap, Activity, AlertCircle, ShieldAlert, CheckCircle2 } from 'lucide-react';

export const GridView: React.FC = () => {
  const [selectedScenarioId, setSelectedScenarioId] = useState<GridScenarioId>('strain');
  const scenario = GRID_SCENARIOS[selectedScenarioId];
  const { config, loading } = useOracle();

  const bountyXlm = config ? (config.bounty_stroops / 10000000).toFixed(1) : '1.0';
  const dailyCapXlm = config ? (config.oracle_daily_cap_stroops / 10000000).toFixed(0) : '10';

  return (
    <div className="space-y-4">
      {/* Simulation Banner Notice */}
      <div
        data-testid="simulated-grid-banner"
        className="p-3 rounded-xl bg-purple-500/10 border border-purple-500/30 text-purple-900 dark:text-purple-200 text-xs flex items-center justify-between"
      >
        <span className="font-semibold uppercase tracking-wider flex items-center gap-1.5">
          <Activity className="w-4 h-4 text-purple-600 dark:text-purple-400" />
          Simulated Municipal Grid Feed
        </span>
        <span className="px-2 py-0.5 rounded-full bg-purple-500/20 text-[10px] font-bold">
          DEMO
        </span>
      </div>

      {/* Scenario Demo Switcher */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-sm transition-colors">
        <label className="text-xs font-semibold text-slate-500 dark:text-slate-400 block mb-2 uppercase tracking-wide">
          Select Simulated Scenario
        </label>
        <div className="grid grid-cols-3 gap-2">
          {(['normal', 'strain', 'emergency'] as GridScenarioId[]).map((id) => {
            const sc = GRID_SCENARIOS[id];
            const isSelected = selectedScenarioId === id;
            return (
              <button
                key={id}
                role="button"
                data-testid={`scenario-btn-${id}`}
                onClick={() => setSelectedScenarioId(id)}
                className={`touch-target px-2 py-2 text-xs font-medium rounded-xl border transition flex flex-col items-center justify-center text-center ${
                  isSelected
                    ? 'border-emerald-500 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 font-semibold'
                    : 'border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'
                }`}
              >
                {sc.name}
              </button>
            );
          })}
        </div>
      </div>

      {/* Main Grid Metrics Card */}
      <div className="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-5 shadow-sm space-y-4 transition-colors">
        <div className="flex items-start justify-between">
          <div>
            <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400">
              <span data-testid="simulated-substation">{scenario.substation}</span>
            </div>
            <h3 className="text-lg font-bold mt-0.5 text-slate-900 dark:text-white">
              {scenario.name}
            </h3>
          </div>

          <span
            className={`px-2.5 py-1 rounded-full text-xs font-semibold inline-flex items-center gap-1 ${
              scenario.severity === 'normal'
                ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
                : scenario.severity === 'warning'
                  ? 'bg-amber-500/10 text-amber-600 dark:text-amber-400'
                  : 'bg-rose-500/10 text-rose-600 dark:text-rose-400'
            }`}
          >
            {scenario.severity === 'normal' ? (
              <CheckCircle2 className="w-3.5 h-3.5" />
            ) : scenario.severity === 'warning' ? (
              <AlertCircle className="w-3.5 h-3.5" />
            ) : (
              <ShieldAlert className="w-3.5 h-3.5" />
            )}
            {scenario.severity.toUpperCase()}
          </span>
        </div>

        {/* Frequency Meter */}
        <div className="p-4 rounded-xl bg-slate-50 dark:bg-slate-950 border border-slate-100 dark:border-slate-800 flex items-center justify-between">
          <div>
            <span className="text-xs text-slate-500 dark:text-slate-400 block font-medium">
              Grid Frequency <span className="text-[10px] text-purple-500">(Simulated)</span>
            </span>
            <span
              data-testid="grid-frequency-value"
              className="text-3xl font-extrabold tracking-tight font-mono text-slate-900 dark:text-white"
            >
              {scenario.frequencyHz.toFixed(2)} Hz
            </span>
          </div>
          <div className="text-right text-xs text-slate-500 dark:text-slate-400">
            <span>Nominal: 50.00 Hz</span>
            <span className="block mt-0.5 text-[11px] opacity-75">Target: 49.80 – 50.20 Hz</span>
          </div>
        </div>

        <p className="text-xs text-slate-600 dark:text-slate-400">{scenario.description}</p>

        {/* Live Oracle Bounty Stats */}
        <div className="pt-2 border-t border-slate-100 dark:border-slate-800 grid grid-cols-2 gap-3">
          <div className="p-3 rounded-xl bg-emerald-500/5 border border-emerald-500/20">
            <span className="text-[11px] font-semibold text-emerald-700 dark:text-emerald-400 block uppercase tracking-wide">
              Verified Payout
            </span>
            <span
              data-testid="live-bounty-value"
              className="text-lg font-bold text-slate-900 dark:text-white mt-0.5 block"
            >
              {loading ? '...' : `${bountyXlm} XLM`}
            </span>
            <span className="text-[10px] text-slate-500 dark:text-slate-400 block">
              Per power-down event
            </span>
          </div>

          <div className="p-3 rounded-xl bg-slate-100 dark:bg-slate-800 border border-slate-200 dark:border-slate-700">
            <span className="text-[11px] font-semibold text-slate-500 dark:text-slate-400 block uppercase tracking-wide">
              Daily Municipal Cap
            </span>
            <span className="text-lg font-bold text-slate-900 dark:text-white mt-0.5 block">
              {loading ? '...' : `${dailyCapXlm} XLM`}
            </span>
            <span className="text-[10px] text-slate-500 dark:text-slate-400 block">
              Max {config?.wallet_daily_claim_limit ?? 2} claims/day
            </span>
          </div>
        </div>
      </div>

      {/* Call to Action Card */}
      <div className="p-4 rounded-2xl bg-gradient-to-r from-emerald-500/10 to-teal-500/10 border border-emerald-500/20 flex items-center justify-between">
        <div>
          <h4 className="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-1.5">
            <Zap className="w-4 h-4 text-emerald-500" />
            Ready to participate?
          </h4>
          <p className="text-xs text-slate-600 dark:text-slate-400 mt-0.5">
            Switch to the Monitor tab to detect your appliance hum.
          </p>
        </div>
      </div>
    </div>
  );
};
