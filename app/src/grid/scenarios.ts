export type GridScenarioId = 'normal' | 'strain' | 'emergency';

export interface GridScenario {
  id: GridScenarioId;
  name: string;
  frequencyHz: number;
  substation: string;
  isStrained: boolean;
  severity: 'normal' | 'warning' | 'critical';
  description: string;
  recommendation: string;
}

export const GRID_SCENARIOS: Record<GridScenarioId, GridScenario> = {
  normal: {
    id: 'normal',
    name: 'Normal Operation',
    frequencyHz: 50.02,
    substation: 'Accra-Central #04 (Simulated)',
    isStrained: false,
    severity: 'normal',
    description: 'Grid frequency is stable across municipal transmission lines.',
    recommendation: 'No action required. Standard tariffs active.',
  },
  strain: {
    id: 'strain',
    name: 'Reserve Strain',
    frequencyHz: 49.82,
    substation: 'Accra-Central #04 (Simulated)',
    isStrained: true,
    severity: 'warning',
    description: 'Peak cooling demand causing regional reserve capacity deficits.',
    recommendation: 'Power down heavy air conditioners and pumps for instant bounty.',
  },
  emergency: {
    id: 'emergency',
    name: 'Critical Curtailment',
    frequencyHz: 49.51,
    substation: 'Accra-Central #04 (Simulated)',
    isStrained: true,
    severity: 'critical',
    description: 'Grid frequency falling rapidly near involuntary shedding threshold.',
    recommendation: 'Immediate appliance shutdown requested by municipal authority.',
  },
};
