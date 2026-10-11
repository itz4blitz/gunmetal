import { useCallback, useEffect, useState } from 'react';
import { addSource, loadSources, removeSource, rescanSource } from './video-api.ts';
import type { SourcesDocument } from './video-types.ts';

/**
 * The Libraries pane of the settings page: the sources this server scans.
 * Name one, give the path the container can see, and the Watch area lists
 * what it finds.
 */
export function SourcesPane() {
  const [sources, setSources] = useState<SourcesDocument>({ sources: [] });
  const [notice, setNotice] = useState('');
  const [name, setName] = useState('');
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
    const added = await addSource(name.trim(), path.trim());
    setBusy(false);
    if (!added.ok) {
      setNotice(added.error);
      return;
    }
    setName('');
    setPath('');
    setNotice('');
    await reload();
  }, [name, path, reload]);

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
    },
    [reload],
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
    },
    [reload],
  );

  return (
    <div id="sources-pane">
      {sources.sources.length === 0 ? (
        <p id="sources-empty">
          No libraries yet. Name one and give the path this container can see, like{' '}
          <code>/media/movies</code> or <code>/media/tvshows</code>.
        </p>
      ) : (
        <ul id="source-list">
          {sources.sources.map((source) => (
            <li key={source.id} data-source-row={source.id}>
              <span className="source-name">{source.name}</span>
              <code className="source-path">{source.path}</code>
              <span className="source-count">
                {source.titles === 1 ? '1 title' : `${source.titles} titles`}
              </span>
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
          Path in this container
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
