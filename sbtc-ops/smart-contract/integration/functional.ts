import { buildDevnetNetworkOrchestrator, getNetworkIdFromEnv } from './helpers-stacking';

export const startOrchestrator = () => {
  let orchestrator = buildDevnetNetworkOrchestrator(getNetworkIdFromEnv());
  console.log('ORCHESTRATOR', orchestrator);
  // orchestrator.start(120000);
  return orchestrator;
};
