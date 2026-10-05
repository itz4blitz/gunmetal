import { spawnSync } from 'node:child_process';
import { check } from '../installation.ts';

// Test support for installation.test.ts. This program is fixed text: nothing is pasted into it.
// Everything that varies between tests reaches it as data, in the JSON of its one argument:
//   directory  the workspace to check
//   request    optional { status, body }: every registry request is answered with that status and JSON body
//   run        optional { argument, answer }: a manager command whose arguments include `argument`
//              is not run and is answered with `answer`
// It prints what the exported check returns, or the findings the check refuses with.
type Answer = { status: number | null; signal: string | null; stdout: string; stderr: string };
type Plan = { directory: string; request?: { status: number; body: unknown }; run?: { argument: string; answer: Answer } };

const plan = JSON.parse(String(process.argv[2])) as Plan;
const { request, run } = plan;
const seams: { request?: typeof fetch; run?: typeof spawnSync } = {};
if (request !== undefined) {
  seams.request = async () => new Response(JSON.stringify(request.body), { status: request.status });
}
if (run !== undefined) {
  seams.run = ((command: string, args: readonly string[], options: object) =>
    args.includes(run.argument) ? run.answer : spawnSync(command, args, options)) as typeof spawnSync;
}
try {
  process.stdout.write(JSON.stringify(await check(plan.directory, seams)));
} catch (error) {
  process.stdout.write(JSON.stringify((error as { findings?: unknown }).findings ?? String(error)));
}
