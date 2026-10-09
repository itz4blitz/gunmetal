export type ActivityJob = {
  id: string;
  label: string;
  done: number;
  total: number;
};

export type Activity = {
  jobs: readonly ActivityJob[];
};

const MAX_JOBS = 8;
const MAX_LABEL = 80;

function job(value: unknown): ActivityJob | undefined {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return undefined;
  }
  const row = value as Record<string, unknown>;
  if (typeof row.id !== 'string' || row.id.length < 1 || row.id.length > 32 || row.id.includes('\0')) {
    return undefined;
  }
  if (typeof row.label !== 'string' || row.label.length < 1 || row.label.length > MAX_LABEL || row.label.includes('\0')) {
    return undefined;
  }
  if (!Number.isInteger(row.done) || !Number.isInteger(row.total)) {
    return undefined;
  }
  const done = row.done as number;
  const total = row.total as number;
  if (done < 0 || total < 1 || done > total || total > 2000) {
    return undefined;
  }
  return { id: row.id, label: row.label, done, total };
}

/** The activity document, or undefined when it is not that shape. */
export function activityFromDocument(value: unknown): Activity | undefined {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return undefined;
  }
  const jobs = (value as Record<string, unknown>).jobs;
  if (!Array.isArray(jobs) || jobs.length > MAX_JOBS) {
    return undefined;
  }
  const parsed: ActivityJob[] = [];
  for (const entry of jobs) {
    const next = job(entry);
    if (next === undefined) {
      return undefined;
    }
    parsed.push(next);
  }
  return { jobs: parsed };
}

/** 0 to 100, from how far the job has got. */
export function activityPercent(row: ActivityJob): number {
  return Math.round((row.done * 100) / row.total);
}

export function activityCount(jobs: readonly ActivityJob[]): number {
  return jobs.reduce((sum, row) => sum + row.done, 0);
}
