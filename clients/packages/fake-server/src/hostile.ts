/** Hostile-metadata corpus payload for the demo album (SEC-API-046 / SEC-CLI-001). */
export function hostileCorpus(): string {
  return '"><img src=x onerror=alert(1)><script>window.__gm_xss=1</script>';
}
