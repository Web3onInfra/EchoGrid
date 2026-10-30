import { http, HttpResponse } from 'msw';
import {
  PublicConfigResponse,
  SelfCheckReport,
  ChallengeResponse,
  ProofConfirmedResponse,
  ClaimPollResponse,
} from './types';

export const mockConfig: PublicConfigResponse = {
  mode: 'mock',
  network: 'testnet',
  contract_id: 'CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF',
  token_id: 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC',
  bounty_stroops: 10000000,
  oracle_daily_cap_stroops: 100000000,
  wallet_daily_claim_limit: 2,
  cooldown_seconds: 600,
  challenge_ttl_seconds: 120,
  simulator_mode: false,
  explorer_base_url: 'https://stellar.expert/explorer/testnet/tx',
};

export const mockReady: SelfCheckReport = {
  rpc_healthy: true,
  oracle_authorized: true,
  contract_paused: false,
  oracle_funded: true,
  contract_id: 'CDFOK4NF4CY27N4G5PYA6SXTJA5MEKPVU7DQRMJUS74LIXHOKYNRHOPF',
  oracle_address: 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
};

export const mockChallenge: ChallengeResponse = {
  challenge_id: 'ch_test_12345678',
  wallet_address: 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
  issued_at: '2026-10-30T00:00:00Z',
  expires_at: '2026-10-30T00:02:00Z',
};

export const mockProofConfirmed: ProofConfirmedResponse = {
  status: 'confirmed',
  claim_id: 'clm_1030_abc123',
  tx_hash: '6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81',
  explorer_url:
    'https://stellar.expert/explorer/testnet/tx/6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81',
  amount_stroops: 10000000,
  simulated: false,
};

export const mockClaimPoll: ClaimPollResponse = {
  claim_id: 'clm_1030_abc123',
  wallet_address: 'GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT',
  status: 'confirmed',
  amount_stroops: 10000000,
  tx_hash: '6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81',
  explorer_url:
    'https://stellar.expert/explorer/testnet/tx/6edcbd9ad86343562aa13bc5e75ab57c649dd8abeb481761738f0609c2a63e81',
  error: null,
  simulated: false,
  created_at: '2026-10-30T00:00:10Z',
  updated_at: '2026-10-30T00:00:25Z',
};

export const handlers = [
  http.get('*/api/v1/health', () => {
    return HttpResponse.json({ status: 'ok', service: 'echogrid-oracle', version: '0.1.0' });
  }),
  http.get('*/api/v1/config', () => {
    return HttpResponse.json(mockConfig);
  }),
  http.get('*/api/v1/ready', () => {
    return HttpResponse.json(mockReady);
  }),
  http.get('*/api/v1/challenge', ({ request }) => {
    const url = new URL(request.url);
    const wallet = url.searchParams.get('wallet');
    if (!wallet || wallet.length !== 56 || !wallet.startsWith('G')) {
      return HttpResponse.json(
        { code: 'INVALID_WALLET_ADDRESS', message: 'Invalid Stellar G... address', status: 400 },
        { status: 400 },
      );
    }
    return HttpResponse.json({ ...mockChallenge, wallet_address: wallet });
  }),
  http.post('*/api/v1/proof', () => {
    return HttpResponse.json(mockProofConfirmed);
  }),
  http.get('*/api/v1/claims/:id', ({ params }) => {
    return HttpResponse.json({ ...mockClaimPoll, claim_id: params.id as string });
  }),
  http.get('https://friendbot.stellar.org*', () => {
    return HttpResponse.json({ result: 'ok' });
  }),
  http.get('https://horizon-testnet.stellar.org/accounts/:id', () => {
    return HttpResponse.json({
      balances: [{ asset_type: 'native', balance: '10000.0000000' }],
    });
  }),
];
