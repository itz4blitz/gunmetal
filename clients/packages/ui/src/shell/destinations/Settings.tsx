import { useState, type ReactNode } from 'react';
import { Text, View } from 'react-native-web';
import type { DestinationMessages } from '../../messages/en/destinations.ts';
import type { ShellMessages } from '../../messages/en/shell.ts';
import { Icon } from '../Icon.tsx';
import type { ThemeId } from '../theme.ts';
import type { WidthClass } from '../width.ts';
import type { PluginSlot } from './settings.ts';
import {
  defaultSettingsSection,
  settingsSectionPath,
  settingsConnectedRows,
  settingsLayout,
  settingsNavItems,
  settingsPlaybackRows,
  settingsSectionTitle,
  settingsSections,
  settingsSwatchLabels,
  settingsThemeForKey,
  type SettingsCrossfadeSeconds,
  type SettingsLevelling,
  type SettingsRowCopy,
  type SettingsSection,
} from './settings.ts';

export type SettingsProps = {
  /** When a route names a section, that pane is the one shown. Not a second settings app. */
  section?: SettingsSection | undefined;
  pluginSlots?: readonly PluginSlot[] | undefined;
  /** Real library size. When set, this is the About library fact, and the playback placeholder is hidden. */
  libraryFact?: string | undefined;
  /** Libraries content. Absent, the Libraries section stays hidden. */
  libraries?: ReactNode | undefined;
  messages: DestinationMessages;
  shellMessages: ShellMessages;
  theme: ThemeId;
  onThemeChange: (theme: ThemeId) => void;
  levelling?: SettingsLevelling | undefined;
  onLevelling?: ((levelling: SettingsLevelling) => void) | undefined;
  crossfadeSeconds?: SettingsCrossfadeSeconds | undefined;
  onCrossfade?: ((seconds: SettingsCrossfadeSeconds) => void) | undefined;
  outputs?: readonly { id: string; label: string }[] | undefined;
  sinkId?: string | undefined;
  onOutput?: ((id: string) => void) | undefined;
  width: WidthClass;
  onOpenPath?: ((path: string) => void) | undefined;
};

function activateKey(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

/** A stable element id. Indexed so an output id of "" or "default" cannot collide. */
function choiceDomId(rowId: string, index: number): string {
  return `settings-${rowId}-choice-${String(index)}`;
}

/**
 * The option an arrow moves to, wrapping at both ends. A current value the
 * group does not offer, or any other key, answers undefined.
 */
function choiceForKey<T extends string | number>(ids: readonly T[], current: T, key: string): T | undefined {
  const index = ids.indexOf(current);
  if (index === -1) {
    return undefined;
  }
  if (key === 'ArrowRight' || key === 'ArrowDown') {
    return ids[(index + 1) % ids.length];
  }
  if (key === 'ArrowLeft' || key === 'ArrowUp') {
    return ids[(index + ids.length - 1) % ids.length];
  }
  return undefined;
}

type Choice<T extends string | number> = { id: T; label: string };

function levellingOptions(messages: DestinationMessages): readonly Choice<SettingsLevelling>[] {
  return [
    { id: 'off', label: messages.settingsPlaybackOff },
    { id: 'track', label: messages.settingsPlaybackTrack },
    { id: 'album', label: messages.settingsPlaybackAlbum },
  ];
}

function crossfadeOptions(messages: DestinationMessages): readonly Choice<SettingsCrossfadeSeconds>[] {
  return [
    { id: 0, label: messages.settingsPlaybackOff },
    { id: 2, label: messages.settingsPlaybackSeconds2 },
    { id: 4, label: messages.settingsPlaybackSeconds4 },
    { id: 6, label: messages.settingsPlaybackSeconds6 },
    { id: 8, label: messages.settingsPlaybackSeconds8 },
    { id: 12, label: messages.settingsPlaybackSeconds12 },
  ];
}

function outputOptions(
  outputs: readonly { id: string; label: string }[],
  defaultLabel: string,
): readonly Choice<string>[] {
  return [{ id: '', label: defaultLabel }, ...outputs];
}

function SettingsPane({
  id,
  section,
  messages,
  children,
}: {
  id: string;
  section: SettingsSection;
  messages: DestinationMessages;
  children: ReactNode;
}) {
  return (
    <View id={id} dataSet={{ settingsSection: section, settingsPanel: '1' }}>
      <View dataSet={{ settingsHeadingRow: '1' }}>
        <Text accessibilityRole="header" dataSet={{ type: 'title2' }}>
          {settingsSectionTitle(section, messages)}
        </Text>
      </View>
      {children}
    </View>
  );
}

/**
 * A setting that is not wired yet: what it is and what it will do on the
 * left, and its status in words on the right. It is never a dead control —
 * nothing here can be pressed or focused.
 */
function UnavailableRow({ row, status }: { row: SettingsRowCopy; status: string }) {
  return (
    <View dataSet={{ settingsRow: row.id, rowState: 'unavailable' }}>
      <View dataSet={{ settingsRowText: '1' }}>
        <Text dataSet={{ settingsRowLabel: '1' }}>{row.label}</Text>
        <Text dataSet={{ settingsRowHint: '1' }}>{row.hint}</Text>
      </View>
      <Text dataSet={{ settingsRowStatus: '1' }}>{status}</Text>
    </View>
  );
}

function ChoiceRadio({
  id,
  label,
  selected,
  tabIndex,
  onSelect,
}: {
  id: string;
  label: string;
  selected: boolean;
  tabIndex: 0 | -1;
  onSelect: () => void;
}) {
  return (
    <View
      id={id}
      accessibilityRole="radio"
      accessibilityLabel={label}
      aria-checked={selected}
      dataSet={{ settingsChoice: '1', selected: selected ? '1' : '0' }}
      tabIndex={tabIndex}
      onClick={onSelect}
      onKeyDown={(event) => {
        activateKey(event, onSelect);
      }}
    >
      <Text dataSet={{ settingsChoiceLabel: '1' }}>{label}</Text>
    </View>
  );
}

function PlaybackChoice<T extends string | number>({
  row,
  options,
  selected,
  onSelect,
}: {
  row: SettingsRowCopy;
  options: readonly Choice<T>[];
  selected: T;
  onSelect: (id: T) => void;
}) {
  const ids = options.map((option) => option.id);
  const checkedIndex = ids.indexOf(selected);
  return (
    <View dataSet={{ settingsRow: row.id }}>
      <View dataSet={{ settingsRowText: '1' }}>
        <Text dataSet={{ settingsRowLabel: '1' }}>{row.label}</Text>
        <Text dataSet={{ settingsRowHint: '1' }}>{row.hint}</Text>
      </View>
      <View
        accessibilityRole="radiogroup"
        accessibilityLabel={row.label}
        dataSet={{ settingsChoices: row.id }}
        onKeyDown={(event) => {
          const next = choiceForKey(ids, selected, event.key);
          if (next === undefined) {
            return;
          }
          event.preventDefault();
          onSelect(next);
          document.getElementById(choiceDomId(row.id, ids.indexOf(next)))?.focus();
        }}
      >
        {options.map((option, index) => (
          <ChoiceRadio
            key={choiceDomId(row.id, index)}
            id={choiceDomId(row.id, index)}
            label={option.label}
            selected={option.id === selected}
            // The checked option is the group's one tab stop. If none is checked, the first is.
            tabIndex={option.id === selected || (checkedIndex === -1 && index === 0) ? 0 : -1}
            onSelect={() => {
              onSelect(option.id);
            }}
          />
        ))}
      </View>
    </View>
  );
}

/** One theme card: a miniature of the app in that theme, drawn by area-settings.css. */
function ThemeSwatch({
  id,
  label,
  selected,
  onSelect,
}: {
  id: ThemeId;
  label: string;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <View
      id={`settings-theme-swatch-${id}`}
      accessibilityRole="radio"
      accessibilityLabel={label}
      aria-checked={selected}
      dataSet={{ themeSwatch: id, selected: selected ? '1' : '0' }}
      // Roving tabindex: the checked card is the group's one tab stop.
      tabIndex={selected ? 0 : -1}
      onClick={onSelect}
      onKeyDown={(event) => {
        activateKey(event, onSelect);
      }}
    >
      <View dataSet={{ swatchStage: '1' }}>
        <View dataSet={{ swatchLines: '1' }} />
        <View dataSet={{ swatchPanel: '1' }}>
          <View dataSet={{ swatchDot: '1' }} />
        </View>
      </View>
      {/* The checked mark: a brass nut on the card's corner, shown by the
          stylesheet on the checked card only. */}
      <View dataSet={{ swatchCheck: '1' }}>
        <Icon name="check" size={12} />
      </View>
      <Text dataSet={{ swatchName: '1' }}>{label}</Text>
    </View>
  );
}

export function Settings({
  section: routedSection,
  libraryFact,
  libraries,
  messages,
  shellMessages,
  theme,
  onThemeChange,
  levelling = 'off',
  onLevelling,
  crossfadeSeconds = 0,
  onCrossfade,
  outputs = [],
  sinkId = '',
  onOutput,
  width,
  onOpenPath,
}: SettingsProps) {
  const layout = settingsLayout(width);
  const [levellingRow, crossfadeRow, outputRow] = settingsPlaybackRows(messages);
  const [picked, setPicked] = useState<{ route: SettingsSection | undefined; section: SettingsSection }>({
    route: routedSection,
    section: routedSection ?? defaultSettingsSection(),
  });
  let section = picked.section;
  if (picked.route !== routedSection) {
    section = routedSection ?? defaultSettingsSection();
    setPicked({ route: routedSection, section });
  }
  const withLibraries = libraries !== undefined;
  const visible = layout === 'stack' ? settingsSections(withLibraries) : [section];
  const choose = (next: SettingsSection) => {
    if (onOpenPath !== undefined) {
      onOpenPath(settingsSectionPath(next));
      return;
    }
    setPicked({ route: routedSection, section: next });
  };
  return (
    <View id="destination-settings" dataSet={{ settingsLayout: layout }}>
      <Text id="destination-headline" accessibilityRole="header">
        {messages.settingsHeadline}
      </Text>
      {layout === 'side' ? (
        <View id="settings-nav" accessibilityRole="tablist" accessibilityLabel={messages.settingsNav}>
          {settingsNavItems(messages, withLibraries).map((item) => (
            <View
              key={item.id}
              id={`settings-nav-${item.id}`}
              accessibilityRole="tab"
              accessibilityLabel={item.label}
              aria-selected={section === item.id}
              dataSet={{ settingsNavItem: item.id, selected: section === item.id ? '1' : '0' }}
              tabIndex={0}
              onClick={() => {
                choose(item.id);
              }}
              onKeyDown={(event) => {
                activateKey(event, () => {
                  choose(item.id);
                });
              }}
            >
              <Text>{item.label}</Text>
            </View>
          ))}
        </View>
      ) : null}
      <View id="settings-panels">
        {visible.includes('appearance') ? (
          <SettingsPane id="settings-appearance" section="appearance" messages={messages}>
            <View dataSet={{ settingsGroup: 'theme' }}>
              <Text dataSet={{ settingsGroupTitle: '1' }}>{shellMessages.themeLabel}</Text>
              <Text dataSet={{ settingsGroupHint: '1' }}>{messages.settingsThemeHint}</Text>
            </View>
            {/* The one theme control: a radio group of preview cards. Arrows
                move the choice and the focus together. */}
            <View
              id="settings-theme-preview"
              accessibilityRole="radiogroup"
              accessibilityLabel={shellMessages.themeLabel}
              onKeyDown={(event) => {
                const next = settingsThemeForKey(theme, event.key);
                if (next === undefined) {
                  return;
                }
                event.preventDefault();
                onThemeChange(next);
                document.getElementById(`settings-theme-swatch-${next}`)?.focus();
              }}
            >
              {settingsSwatchLabels(shellMessages).map(({ id, label }) => (
                <ThemeSwatch
                  key={id}
                  id={id}
                  label={label}
                  selected={id === theme}
                  onSelect={() => {
                    onThemeChange(id);
                  }}
                />
              ))}
            </View>
          </SettingsPane>
        ) : null}
        {visible.includes('playback') ? (
          <SettingsPane id="settings-playback" section="playback" messages={messages}>
            <View dataSet={{ settingsRows: 'playback' }}>
              <PlaybackChoice
                row={levellingRow}
                options={levellingOptions(messages)}
                selected={levelling}
                onSelect={(id) => {
                  onLevelling?.(id);
                }}
              />
              <PlaybackChoice
                row={crossfadeRow}
                options={crossfadeOptions(messages)}
                selected={crossfadeSeconds}
                onSelect={(id) => {
                  onCrossfade?.(id);
                }}
              />
              <PlaybackChoice
                row={outputRow}
                options={outputOptions(outputs, messages.settingsPlaybackDefault)}
                selected={sinkId}
                onSelect={(id) => {
                  onOutput?.(id);
                }}
              />
            </View>
            {libraryFact === undefined ? (
              <Text dataSet={{ settingsPlaceholder: 'playback', settingsNote: '1' }}>
                {messages.settingsPlaybackPlaceholder}
              </Text>
            ) : null}
          </SettingsPane>
        ) : null}
        {visible.includes('libraries') && libraries !== undefined ? (
          <SettingsPane id="settings-libraries" section="libraries" messages={messages}>
            {libraries}
          </SettingsPane>
        ) : null}
        {visible.includes('connected') ? (
          <SettingsPane id="settings-connected" section="connected" messages={messages}>
            <View dataSet={{ settingsRows: 'connected' }}>
              {settingsConnectedRows(messages).map((row) => (
                <UnavailableRow key={row.id} row={row} status={messages.settingsUnavailable} />
              ))}
            </View>
            <Text dataSet={{ emptyState: 'connected', settingsNote: '1' }}>{messages.settingsConnectedEmpty}</Text>
          </SettingsPane>
        ) : null}
        {visible.includes('about') ? (
          <SettingsPane id="settings-about" section="about" messages={messages}>
            <View id="settings-about-facts" dataSet={{ settingsFacts: '1' }}>
              <View dataSet={{ settingsFact: 'data' }}>
                <Text dataSet={{ factLabel: '1' }}>{messages.settingsAboutDataLabel}</Text>
                <Text dataSet={{ factValue: '1' }}>
                  {libraryFact === undefined ? messages.settingsAboutData : libraryFact}
                </Text>
              </View>
              <View dataSet={{ settingsFact: 'address' }}>
                <Text dataSet={{ factLabel: '1' }}>{messages.settingsAboutAddressLabel}</Text>
                <Text dataSet={{ factValue: '1' }}>{messages.settingsAboutAddress}</Text>
              </View>
              <View dataSet={{ settingsFact: 'version' }}>
                <Text dataSet={{ factLabel: '1' }}>{messages.settingsAboutVersionLabel}</Text>
                <Text dataSet={{ factValue: '1' }}>{messages.settingsAboutVersion}</Text>
              </View>
            </View>
          </SettingsPane>
        ) : null}
        {visible.includes('privacy') ? (
          <SettingsPane id="settings-privacy" section="privacy" messages={messages}>
            <View dataSet={{ settingsRows: 'privacy' }}>
              <Text dataSet={{ settingsPrivacy: '1', settingsStatement: '1' }}>{messages.settingsPrivacyBody}</Text>
            </View>
          </SettingsPane>
        ) : null}
      </View>
    </View>
  );
}
