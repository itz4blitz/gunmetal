/**
 * The owner's choice for each store record, on this server.
 *
 * Install and uninstall write that choice. They do not fetch a package
 * and they do not run one (SEC-EXT-018). A record this build does not
 * run stays not running, even when the choice says installed.
 */

export const EXTENSION_CHOICES_KEY = 'gunmetal.extension.choices';

export type ExtensionSettingKind = 'toggle' | 'choice';

export type ExtensionSetting = {
  id: string;
  label: string;
  kind: ExtensionSettingKind;
  options: readonly { id: string; label: string }[];
  defaultValue: string;
};

const TOGGLE: readonly { id: string; label: string }[] = [
  { id: 'on', label: 'On' },
  { id: 'off', label: 'Off' },
];

const SETTINGS: Record<string, readonly ExtensionSetting[]> = {
  'cover-art': [
    { id: 'archive', label: 'Cover Art Archive', kind: 'toggle', options: TOGGLE, defaultValue: 'on' },
    { id: 'portraits', label: 'Artist photos', kind: 'toggle', options: TOGGLE, defaultValue: 'on' },
  ],
  'url-style': [
    {
      id: 'style',
      label: 'Address style',
      kind: 'choice',
      options: [
        { id: 'slug', label: 'Name' },
        { id: 'id', label: 'Id' },
      ],
      defaultValue: 'slug',
    },
  ],
  scrobble: [
    {
      id: 'service',
      label: 'Service',
      kind: 'choice',
      options: [
        { id: 'none', label: 'None' },
        { id: 'listenbrainz', label: 'ListenBrainz' },
        { id: 'lastfm', label: 'Last.fm' },
      ],
      defaultValue: 'none',
    },
  ],
  'home-rows': [
    {
      id: 'row',
      label: 'Row',
      kind: 'choice',
      options: [
        { id: 'recent', label: 'Recently added' },
        { id: 'loved', label: 'Loved' },
      ],
      defaultValue: 'recent',
    },
  ],
};

export type ExtensionChoice = {
  installed: boolean;
  values: Record<string, string>;
};

export type ExtensionChoices = Record<string, ExtensionChoice>;

export function settingsFor(id: string): readonly ExtensionSetting[] {
  return SETTINGS[id] ?? [];
}

export function defaultChoice(id: string, status: 'on' | 'not-in-build'): ExtensionChoice {
  const values: Record<string, string> = {};
  for (const setting of settingsFor(id)) {
    values[setting.id] = setting.defaultValue;
  }
  return { installed: status === 'on', values };
}

function choiceRecord(value: unknown, id: string, status: 'on' | 'not-in-build'): ExtensionChoice {
  const base = defaultChoice(id, status);
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return base;
  }
  const record = value as Record<string, unknown>;
  const values = { ...base.values };
  if (typeof record.values === 'object' && record.values !== null && !Array.isArray(record.values)) {
    for (const setting of settingsFor(id)) {
      const next = (record.values as Record<string, unknown>)[setting.id];
      if (typeof next === 'string' && setting.options.some((option) => option.id === next)) {
        values[setting.id] = next;
      }
    }
  }
  return {
    installed: typeof record.installed === 'boolean' ? record.installed : base.installed,
    values,
  };
}

/** Read saved choices. Anything unreadable is the default for that id. */
export function parseChoices(raw: string | null, ids: readonly { id: string; status: 'on' | 'not-in-build' }[]): ExtensionChoices {
  let parsed: unknown = undefined;
  if (raw !== null && raw !== '') {
    try {
      parsed = JSON.parse(raw) as unknown;
    } catch {
      parsed = undefined;
    }
  }
  const saved = typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed) ? (parsed as Record<string, unknown>) : {};
  const choices: ExtensionChoices = {};
  for (const entry of ids) {
    choices[entry.id] = choiceRecord(saved[entry.id], entry.id, entry.status);
  }
  return choices;
}

export function installChoice(choices: ExtensionChoices, id: string, status: 'on' | 'not-in-build'): ExtensionChoices {
  const current = choices[id] ?? defaultChoice(id, status);
  return { ...choices, [id]: { ...current, installed: true } };
}

export function uninstallChoice(choices: ExtensionChoices, id: string, status: 'on' | 'not-in-build'): ExtensionChoices {
  const current = choices[id] ?? defaultChoice(id, status);
  return { ...choices, [id]: { ...current, installed: false } };
}

export function setChoiceValue(
  choices: ExtensionChoices,
  id: string,
  status: 'on' | 'not-in-build',
  settingId: string,
  value: string,
): ExtensionChoices {
  const setting = settingsFor(id).find((item) => item.id === settingId);
  if (setting === undefined || !setting.options.some((option) => option.id === value)) {
    return choices;
  }
  const current = choices[id] ?? defaultChoice(id, status);
  return { ...choices, [id]: { ...current, values: { ...current.values, [settingId]: value } } };
}

export function serializeChoices(choices: ExtensionChoices): string {
  return JSON.stringify(choices);
}
