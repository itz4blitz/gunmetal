import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import { activityCount, activityFromDocument, activityPercent, type ActivityJob } from './activity.ts';

export type ActivityBarProps = {
  onAdvance?: (done: number) => void;
  fetchImpl?: typeof fetch;
};

/** A Plex-style job card. Hidden when the host has nothing running. */
export function ActivityBar({ onAdvance, fetchImpl = globalThis.fetch }: ActivityBarProps) {
  const [jobs, setJobs] = useState<readonly ActivityJob[]>([]);

  useEffect(() => {
    let stop = false;
    let seen = 0;
    const tick = async () => {
      try {
        const response = await fetchImpl('/activity.json');
        if (!response.ok) {
          return;
        }
        const parsed = activityFromDocument(await response.json());
        if (stop || parsed === undefined) {
          return;
        }
        const done = activityCount(parsed.jobs);
        if (done !== seen) {
          seen = done;
          onAdvance?.(done);
        }
        if (!stop) {
          setJobs(parsed.jobs);
        }
      } catch {
        if (!stop) {
          setJobs([]);
        }
      }
    };
    void tick();
    const timer = globalThis.setInterval(() => {
      void tick();
    }, 2000);
    return () => {
      stop = true;
      globalThis.clearInterval(timer);
    };
  }, [fetchImpl, onAdvance]);

  if (jobs.length === 0) {
    return null;
  }
  return (
    <View id="activity-bar" accessibilityRole="status" accessibilityLabel="Library activity">
      {jobs.map((row) => (
        <View key={row.id} dataSet={{ activityJob: row.id }}>
          <Text dataSet={{ activityLabel: '1' }}>{row.label}</Text>
          <View dataSet={{ activityTrack: '1' }}>
            <View dataSet={{ activityFill: '1' }} style={{ width: `${activityPercent(row)}%` }} />
          </View>
          <Text dataSet={{ activityCount: '1' }}>{`${row.done} of ${row.total}`}</Text>
        </View>
      ))}
    </View>
  );
}
