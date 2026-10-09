/**
 * The extensions store. A merged pull request lists a record here.
 * The page reads the same-origin index. It does not import or run a
 * package (SEC-EXT-018).
 */
import { useEffect, useState } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import {
  EXTENSION_CHOICES_KEY,
  installChoice,
  parseChoices,
  savedChoicesRaw,
  serializeChoices,
  setChoiceValue,
  settingsFor,
  uninstallChoice,
  type ExtensionChoices,
} from '../../plugins/extension-choices.ts';
import { extensionById, extensionRepository, storeDetailPath } from '../../plugins/repository.ts';
import { loadPublishedStore, type StoreListing } from '../../plugins/store-catalog.ts';

export type StoreProps = {
  messages: DestinationMessages;
  onOpenPath?: ((path: string) => void) | undefined;
  /** A record open inside the store. Absent means the catalogue. */
  selectedId?: string | undefined;
  /** A test or a caller can pass the published list. Absent means load it. */
  listings?: readonly StoreListing[] | undefined;
};

function activateKey(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

/** The short label a record wears on a card or a detail heading. */
function recordState(status: 'on' | 'not-in-build', installed: boolean, messages: DestinationMessages): string {
  if (installed && status === 'on') {
    return messages.storeOn;
  }
  if (installed) {
    return messages.storeSaved;
  }
  if (status === 'on') {
    return messages.settingsExtensionOff;
  }
  return messages.storeCatalogue;
}

/** The record's own knobs, saved on this server. Only records with knobs get any. */
function SettingControls({
  id,
  status,
  choices,
  messages,
  remember,
}: {
  id: string;
  status: 'on' | 'not-in-build';
  choices: ExtensionChoices;
  messages: DestinationMessages;
  remember: (next: ExtensionChoices) => void;
}) {
  const settings = settingsFor(id);
  if (settings.length === 0) {
    return null;
  }
  return (
    <View dataSet={{ storeSettings: '1' }}>
      {settings.map((setting) => (
        <View key={setting.id} dataSet={{ storeSetting: setting.id }}>
          <Text dataSet={{ storeSettingLabel: '1' }}>{setting.label}</Text>
          <View dataSet={{ storeSettingOptions: setting.id }}>
            {setting.options.map((option) => {
              const selected = choices[id]?.values[setting.id] === option.id;
              const choose = () => {
                remember(setChoiceValue(choices, id, status, setting.id, option.id));
              };
              return (
                <View
                  key={option.id}
                  dataSet={{ storeSettingOption: option.id, storeSettingSelected: selected ? '1' : '0' }}
                  accessibilityRole="button"
                  accessibilityLabel={`${setting.label} ${option.label}`}
                  tabIndex={0}
                  onClick={choose}
                  onKeyDown={(event) => {
                    activateKey(event, choose);
                  }}
                >
                  <Text>{option.label}</Text>
                </View>
              );
            })}
          </View>
        </View>
      ))}
    </View>
  );
}

export function Store({ messages, onOpenPath, selectedId, listings }: StoreProps) {
  const builtIn = extensionRepository().extensions;
  const [published, setPublished] = useState<readonly StoreListing[] | undefined>(listings);
  useEffect(() => {
    if (listings !== undefined) {
      setPublished(listings);
      return;
    }
    /* In a test or a runtime without fetch, loadPublishedStore answers
       undefined and the built-in list stays. */
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
  const listed = shown.map((entry) => ({
    id: entry.id,
    status: entry.status === 'on' ? ('on' as const) : ('not-in-build' as const),
  }));
  const [savedChoices, setSavedChoices] = useState<ExtensionChoices>(() => parseChoices(savedChoicesRaw(), listed));
  /* Saved values are re-read against the whole shown list, so settings for an
     id the published index added survive a toggle of some other record. */
  const choices: ExtensionChoices = { ...parseChoices(savedChoicesRaw(), listed), ...savedChoices };
  const remember = (next: ExtensionChoices) => {
    setSavedChoices(next);
    try {
      localStorage.setItem(EXTENSION_CHOICES_KEY, serializeChoices(next));
    } catch {
      // The choice still shows when the browser cannot store it.
    }
  };
  const selected = selectedId === undefined ? undefined : shown.find((entry) => entry.id === selectedId);
  if (selected !== undefined) {
    const installed = choices[selected.id]?.installed === true;
    const on = extensionById(selected.id)?.status === 'on';
    const status = on ? ('on' as const) : ('not-in-build' as const);
    const toggle = () => {
      remember(installed ? uninstallChoice(choices, selected.id, status) : installChoice(choices, selected.id, status));
    };
    const back = () => {
      onOpenPath?.('/store');
    };
    return (
      <View id="destination-store" dataSet={{ storeDetail: selected.id }}>
        <View
          dataSet={{ storeBack: '1' }}
          accessibilityRole="button"
          accessibilityLabel="Back to store"
          tabIndex={0}
          onClick={back}
          onKeyDown={(event) => {
            activateKey(event, back);
          }}
        >
          <Text>Back to store</Text>
        </View>
        <View dataSet={{ storeDetailBody: '1' }}>
          <View dataSet={{ storeTitleRow: '1' }}>
            <Text id="destination-headline" accessibilityRole="header">
              {selected.title}
            </Text>
            <Text dataSet={{ storeStatus: installed && on ? 'on' : 'catalogue' }}>
              {recordState(status, installed, messages)}
            </Text>
          </View>
          <Text dataSet={{ storeSummary: '1' }}>{selected.summary}</Text>
          <View dataSet={{ storeDoes: '1' }}>
            <Text dataSet={{ storeSectionTitle: '1' }}>{messages.settingsExtensionDoes}</Text>
            {selected.detail.map((line) => (
              <Text key={line} dataSet={{ storeDetailLine: '1' }}>
                {line}
              </Text>
            ))}
          </View>
          <View dataSet={{ storeActionRow: '1' }}>
            <View
              dataSet={{ storeActionDetail: installed ? 'uninstall' : 'install' }}
              accessibilityRole="button"
              accessibilityLabel={`${installed ? messages.storeUninstall : messages.storeInstall} ${selected.title}`}
              tabIndex={0}
              onClick={toggle}
              onKeyDown={(event) => {
                activateKey(event, toggle);
              }}
            >
              <Text>{installed ? messages.storeUninstall : messages.storeInstall}</Text>
            </View>
            {installed && !on ? <Text dataSet={{ storeSavedNote: '1' }}>{messages.settingsExtensionSaved}</Text> : null}
          </View>
          <SettingControls id={selected.id} status={status} choices={choices} messages={messages} remember={remember} />
        </View>
      </View>
    );
  }
  return (
    <View id="destination-store">
      <Text id="destination-headline" accessibilityRole="header">
        {messages.storeHeadline}
      </Text>
      <View dataSet={{ storeGrid: '1' }}>
        {shown.map((entry) => {
          /* The local build is the truth for what runs here; the published
             field only describes the record. */
          const on = extensionById(entry.id)?.status === 'on';
          const installed = choices[entry.id]?.installed === true;
          const known = extensionById(entry.id);
          const openKnown =
            known === undefined
              ? undefined
              : () => {
                  onOpenPath?.(storeDetailPath(known.id));
                };
          const toggle = () => {
            const status = on ? 'on' : 'not-in-build';
            remember(installed ? uninstallChoice(choices, entry.id, status) : installChoice(choices, entry.id, status));
          };
          return (
            <View key={entry.id} dataSet={{ storeItem: entry.id }}>
              <View
                dataSet={{ storeCard: entry.id }}
                accessibilityRole={known === undefined ? undefined : 'button'}
                accessibilityLabel={entry.title}
                tabIndex={known === undefined ? undefined : 0}
                onClick={openKnown}
                onKeyDown={
                  openKnown === undefined
                    ? undefined
                    : (event) => {
                        activateKey(event, openKnown);
                      }
                }
              >
                <Text dataSet={{ storeTitle: '1' }}>{entry.title}</Text>
                <Text dataSet={{ storeSummary: '1' }}>{entry.summary}</Text>
                <Text dataSet={{ storeStatus: installed && on ? 'on' : 'catalogue' }}>
                  {recordState(on ? 'on' : 'not-in-build', installed, messages)}
                </Text>
              </View>
              <View
                dataSet={{ storeAction: installed ? 'uninstall' : 'install' }}
                accessibilityRole="button"
                accessibilityLabel={`${installed ? messages.storeUninstall : messages.storeInstall} ${entry.title}`}
                tabIndex={0}
                onClick={toggle}
                onKeyDown={(event) => {
                  activateKey(event, toggle);
                }}
              >
                <Text>{installed ? messages.storeUninstall : messages.storeInstall}</Text>
              </View>
            </View>
          );
        })}
      </View>
    </View>
  );
}
