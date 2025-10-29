export function resolveJobLandingRoute(hasActiveJob: boolean, queueLength: number): '/progress' | '/mods' {
  return hasActiveJob || queueLength > 0 ? '/progress' : '/mods'
}

export default resolveJobLandingRoute
