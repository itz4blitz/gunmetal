/**
 * The extensions store. A merged pull request lists a record here.
 * The page reads the same-origin index. It does not import or run a
 * package (SEC-EXT-018).
 */
import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import { extensionById, extensionPath, extensionRepository, officialExtensionsRepository } from '../../plugins/repository.ts';
import { loadPublishedStore, type StoreListing } from '../../plugins/store-catalog.ts';

export type StoreProps = {
  messages: DestinationMessages;
  onOpenPath?: ((path: string) => void) | undefined;
  /** A test or a caller can pass the published list. Absent means load it. */
  listings?: readonly StoreListing[] | undefined;
};

function activateKey(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

export function Store({ messages, onOpenPath, listings }: StoreProps) {
  const home = officialExtensionsRepository();
  const builtIn = extensionRepository().extensions;
  const [published, setPublished] = useState<readonly StoreListing[] | undefined>(listings);
  useEffect(() => {
    if (listings !== undefined) {
      setPublished(listings);
      return;
    }
    if (typeof process !== 'undefined' && process.env.VITEST === 'true') {
      return;
    }
    let cancelled = false;
    void loadPublishedStore().then((next) => {
      if (!cancelled && next !== undefined) {
        setPublished(next);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [listings]);
  const shown = published ?? builtIn;
  return (
    <View id="destination-store">
      <Text id="destination-headline" accessibilityRole="header">
        Store
      </Text>
      <Text dataSet={{ storeLede: '1' }}>{messages.storeLede}</Text>
      <Text dataSet={{ storeHome: '1' }}>{home.path}</Text>
      <View dataSet={{ storeGrid: '1' }}>
        {shown.map((entry) => {
          const on = entry.status === 'on';
          const known = extensionById(entry.id);
          const open = () => {
            if (known === undefined) {
              return;
            }
            onOpenPath?.(extensionPath(known.id));
          };
          return (
            <View
              key={entry.id}
              dataSet={{ storeCard: entry.id }}
              accessibilityRole="button"
              accessibilityLabel={entry.title}
              tabIndex={0}
              onClick={open}
              onKeyDown={(event) => {
                activateKey(event, open);
              }}
            >
              <Text dataSet={{ storeTitle: '1' }}>{entry.title}</Text>
              <Text dataSet={{ storeSummary: '1' }}>{entry.summary}</Text>
              <Text dataSet={{ storeStatus: on ? 'on' : 'catalogue' }}>
                {on ? messages.storeOn : messages.storeCatalogue}
              </Text>
            </View>
          );
        })}
      </View>
    </View>
  );
}
