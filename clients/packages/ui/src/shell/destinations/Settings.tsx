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
  settingsConnectedRows,
  settingsLayout,
  settingsNavItems,
  settingsPlaybackRows,
  settingsRelease,
  settingsSectionTitle,
  settingsSections,
  settingsSlotPlaneLabel,
  settingsSlotTitle,
  settingsSwatchLabels,
  settingsThemeForKey,
  type SettingsRowCopy,
  type SettingsSection,
} from './settings.ts';

export type SettingsProps = {
  pluginSlots?: readonly PluginSlot[] | undefined;
  messages: DestinationMessages;
  shellMessages: ShellMessages;
  theme: ThemeId;
  onThemeChange: (theme: ThemeId) => void;
  width: WidthClass;
};

function activateKey(event: { key: string; preventDefault: () => void }, action: () => void): void {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    action();
  }
}

function ReleaseBadge({ section, messages }: { section: SettingsSection; messages: DestinationMessages }) {
  const release = settingsRelease(section);
  return (
    <Text dataSet={{ settingsBadge: release }}>
      {release === 'R1' ? messages.settingsBadgeR1 : messages.settingsBadgeR2}
    </Text>
  );
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
        <ReleaseBadge section={section} messages={messages} />
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

export function Settings({ pluginSlots = [], messages, shellMessages, theme, onThemeChange, width }: SettingsProps) {
  const layout = settingsLayout(width);
  const [section, setSection] = useState<SettingsSection>(defaultSettingsSection());
  const visible = layout === 'stack' ? settingsSections() : [section];

  return (
    <View id="destination-settings" dataSet={{ settingsLayout: layout }}>
      <Text id="destination-headline" accessibilityRole="header">
        {messages.settingsHeadline}
      </Text>
      {layout === 'side' ? (
        <View id="settings-nav" accessibilityRole="tablist" accessibilityLabel={messages.settingsNav}>
          {settingsNavItems(messages).map((item) => (
            <View
              key={item.id}
              id={`settings-nav-${item.id}`}
              accessibilityRole="tab"
              accessibilityLabel={item.label}
              aria-selected={section === item.id}
              dataSet={{ settingsNavItem: item.id, selected: section === item.id ? '1' : '0' }}
              tabIndex={0}
              onClick={() => {
                setSection(item.id);
              }}
              onKeyDown={(event) => {
                activateKey(event, () => {
                  setSection(item.id);
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
              {settingsPlaybackRows(messages).map((row) => (
                <UnavailableRow key={row.id} row={row} status={messages.settingsUnavailable} />
              ))}
            </View>
            <Text dataSet={{ settingsPlaceholder: 'playback', settingsNote: '1' }}>
              {messages.settingsPlaybackPlaceholder}
            </Text>
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
        {visible.includes('extensions') ? (
          <SettingsPane id="settings-extensions" section="extensions" messages={messages}>
            <View dataSet={{ settingsRows: 'extensions' }}>
              <Text dataSet={{ emptyState: 'extensions', settingsStatement: '1' }}>
                {messages.settingsExtensionsBody}
              </Text>
            </View>
            {/* A table of the slots a plugin could fill. With no slots to
                list there is no table: a column head over nothing reads as
                broken. */}
            {pluginSlots.length === 0 ? null : (
              <View id="settings-plugin-slots">
                <View dataSet={{ slotHead: '1' }}>
                  <View dataSet={{ slotHeadLead: '1' }} />
                  <Text dataSet={{ slotHeadTitle: '1' }}>{messages.settingsSlotColumnSlot}</Text>
                  <Text dataSet={{ slotHeadPlane: '1' }}>{messages.settingsSlotColumnPlane}</Text>
                  <Text dataSet={{ slotHeadVersion: '1' }}>{messages.settingsSlotColumnVersion}</Text>
                  <Text dataSet={{ slotHeadState: '1' }}>{messages.settingsSlotColumnState}</Text>
                </View>
                {pluginSlots.map((slot) => {
                  // The row says what the slot is, never a fixed word.
                  const state = slot.loaded
                    ? { flag: '1', label: messages.settingsSlotLoaded }
                    : { flag: '0', label: messages.settingsSlotUnloaded };
                  return (
                    <View
                      key={slot.id}
                      dataSet={{ pluginSlot: slot.id, slotLoaded: state.flag, slotPlane: slot.plane }}
                    >
                      <View dataSet={{ slotMark: '1' }} />
                      <Text dataSet={{ slotTitle: slot.id }}>{settingsSlotTitle(slot.id, messages)}</Text>
                      <Text dataSet={{ slotPlane: slot.plane }}>{settingsSlotPlaneLabel(slot.plane, messages)}</Text>
                      <Text dataSet={{ slotVersion: '1' }}>
                        {slot.manifest === undefined ? '—' : slot.manifest.version}
                      </Text>
                      <Text dataSet={{ slotState: state.flag }}>{state.label}</Text>
                    </View>
                  );
                })}
              </View>
            )}
          </SettingsPane>
        ) : null}
        {visible.includes('about') ? (
          <SettingsPane id="settings-about" section="about" messages={messages}>
            <View id="settings-about-facts" dataSet={{ settingsFacts: '1' }}>
              <View dataSet={{ settingsFact: 'data' }}>
                <Text dataSet={{ factLabel: '1' }}>{messages.settingsAboutDataLabel}</Text>
                <Text dataSet={{ factValue: '1' }}>{messages.settingsAboutData}</Text>
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
