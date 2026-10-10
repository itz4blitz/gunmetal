import { useCallback, useEffect, useState } from 'react';
import { addSource, loadSources, removeSource, rescanSource } from './video-api.ts';
import type { LibraryKind, SourcesDocument } from './video-types.ts';

export type SourcesPaneProps = {
  /** Called after the server accepted an add, a rescan or a remove. */
  onChange?: (() => void) | undefined;
};

/**
 * The Libraries pane of the settings page: every folder this server reads.
 * A library is a type, a name and a folder on the server. Music fills the
 * library and the player; movies and TV shows fill the Watch area. Nothing
 * is read that is not listed here.
 */
export function SourcesPane({ onChange }: SourcesPaneProps) {
  const [sources, setSources] = useState<SourcesDocument>({ sources: [] });
  const [notice, setNotice] = useState('');
  const [name, setName] = useState('');
  const [kind, setKind] = useState<LibraryKind>('music');
  const [path, setPath] = useState('');
  const [busy, setBusy] = useState(false);

  const reload = useCallback(async () => {
    const read = await loadSources();
    setSources(read.ok ? read.value : { sources: [] });
    setNotice(read.ok ? '' : read.error);
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const onAdd = useCallback(async () => {
    setBusy(true);
    const added = await addSource(name.trim(), path.trim(), kind);
    setBusy(false);
    if (!added.ok) {
      setNotice(added.error);
      return;
    }
    setName('');
    setKind('music');
    setPath('');
    setNotice('');
    await reload();
    onChange?.();
  }, [name, kind, path, reload, onChange]);

  const onRemove = useCallback(
    async (id: string) => {
      setBusy(true);
      const removed = await removeSource(id);
      setBusy(false);
      if (!removed.ok) {
        setNotice(removed.error);
        return;
      }
      await reload();
      onChange?.();
    },
    [reload, onChange],
  );

  const onRescan = useCallback(
    async (id: string) => {
      setBusy(true);
      const rescanned = await rescanSource(id);
      setBusy(false);
      if (!rescanned.ok) {
        setNotice(rescanned.error);
        return;
      }
      await reload();
      onChange?.();
    },
    [reload, onChange],
  );

  return (
    <div id="sources-pane">
      {sources.sources.length === 0 ? (
        <p id="sources-empty">
          No libraries yet. Pick a type, name it and give the folder on the server, like <code>/media/music</code> or{' '}
          <code>/media/movies</code>.
        </p>
      ) : (
        <ul id="source-list">
          {sources.sources.map((source) => (
            <li key={source.id} data-source-row={source.id}>
              <span className="source-name">{source.name}</span>
              <span className="source-kind" data-source-kind={source.kind}>
                {kindLabel(source.kind)}
              </span>
              <code className="source-path">{source.path}</code>
              <span className="source-count">{countLabel(source.kind, source.items)}</span>
              <button type="button" disabled={busy} onClick={() => void onRescan(source.id)}>
                Rescan
              </button>
              <button type="button" disabled={busy} onClick={() => void onRemove(source.id)}>
                Remove
              </button>
            </li>
          ))}
        </ul>
      )}
      <form
        id="source-add"
        onSubmit={(event) => {
          event.preventDefault();
          void onAdd();
        }}
      >
        <label>
          Type
          <select value={kind} onChange={(event) => setKind(kindFromSelect(event.target.value))} required>
            <option value="music">Music</option>
            <option value="movies">Movies</option>
            <option value="shows">TV shows</option>
          </select>
        </label>
        <label>
          Name
          <input
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="Movies"
            required
            maxLength={80}
          />
        </label>
        <label>
          Folder on the server
          <input
            value={path}
            onChange={(event) => setPath(event.target.value)}
            placeholder="/media/movies"
            required
            maxLength={200}
            pattern="/.*"
          />
        </label>
        <button type="submit" disabled={busy}>
          Add library
        </button>
      </form>
      {notice !== '' && (
        <p id="source-notice" role="alert">
          {notice}
        </p>
      )}
    </div>
  );
}

function kindLabel(kind: LibraryKind): string {
  if (kind === 'music') {
    return 'Music';
  }
  if (kind === 'movies') {
    return 'Movies';
  }
  return 'TV shows';
}

function countLabel(kind: LibraryKind, items: number): string {
  const unit = kind === 'music' ? 'track' : 'title';
  return items === 1 ? `1 ${unit}` : `${items} ${unit}s`;
}

function kindFromSelect(value: string): LibraryKind {
  if (value === 'movies' || value === 'shows') {
    return value;
  }
  return 'music';
}
