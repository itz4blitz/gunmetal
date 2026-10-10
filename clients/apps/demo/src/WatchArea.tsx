import { useCallback, useEffect, useState } from 'react';
import {
  addSource,
  loadSources,
  loadVideoDocument,
  removeSource,
  rescanSource,
} from './video-api.ts';
import { browserPlayable, type SourcesDocument, type VideoDocument, type VideoTitle } from './video-types.ts';

/** One line of meta under a title: the show and numbers, or the size. */
function metaOf(title: VideoTitle): string {
  const size =
    title.bytes >= 1_000_000_000
      ? `${(title.bytes / 1_000_000_000).toFixed(1)} GB`
      : `${Math.round(title.bytes / 1_000_000)} MB`;
  if (title.kind === 'episode') {
    return `${title.show} · S${title.season} E${title.episode} · ${size}`;
  }
  return `Film · ${size}`;
}

const badge = (title: VideoTitle): string => (title.kind === 'episode' ? `S${title.season} E${title.episode}` : 'Film');

export function WatchArea() {
  const [sources, setSources] = useState<SourcesDocument>({ sources: [] });
  const [document, setDocument] = useState<VideoDocument>({ kind: 'video', titles: [] });
  const [notice, setNotice] = useState('');
  const [name, setName] = useState('');
  const [path, setPath] = useState('');
  const [busy, setBusy] = useState(false);
  const [selected, setSelected] = useState<VideoTitle | undefined>(undefined);

  const reload = useCallback(async () => {
    const [readSources, readDocument] = await Promise.all([loadSources(), loadVideoDocument()]);
    setSources(readSources.ok ? readSources.value : { sources: [] });
    setDocument(readDocument.ok ? readDocument.value : { kind: 'video', titles: [] });
    setNotice(
      readSources.ok && readDocument.ok
        ? ''
        : readSources.ok
          ? (readDocument as { ok: false; error: string }).error
          : (readSources as { ok: false; error: string }).error,
    );
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
    <section id="watch-area" aria-label="Watch">
      <header>
        <h1>Watch</h1>
        <p>Films and episodes, from the sources this server scans.</p>
      </header>

      <div id="watch-sources">
        <h2>Sources</h2>
        {sources.sources.length === 0 ? (
          <p id="watch-sources-empty">
            No sources yet. Name one and give the path this container can see, like{' '}
            <code>/media/movies</code> or <code>/media/tvshows</code>.
          </p>
        ) : (
          <ul id="watch-source-list">
            {sources.sources.map((source) => (
              <li key={source.id}>
                <span className="watch-source-name">{source.name}</span>
                <code className="watch-source-path">{source.path}</code>
                <span className="watch-source-count">
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
          id="watch-add-source"
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
            Add source
          </button>
        </form>
        {notice !== '' && (
          <p id="watch-notice" role="alert">
            {notice}
          </p>
        )}
      </div>

      {document.titles.length === 0 ? (
        <p id="watch-grid-empty">
          {sources.sources.length === 0
            ? 'Add a source to see your media here.'
            : 'No video files found in the configured sources yet.'}
        </p>
      ) : (
        <ul id="watch-grid">
          {document.titles.map((title) => (
            <li key={title.id}>
              <button type="button" className="watch-title" onClick={() => setSelected(title)}>
                <span className="watch-badge">{badge(title)}</span>
                <span className="watch-title-name">{title.title}</span>
                <span className="watch-title-meta">{metaOf(title)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}

      {selected !== undefined && (
        <div id="watch-player">
          <button type="button" id="watch-player-close" onClick={() => setSelected(undefined)}>
            Close
          </button>
          <h3>{selected.title}</h3>
          <p>{metaOf(selected)} · {selected.container}</p>
          {browserPlayable(selected.container) ? (
            <video controls preload="metadata" src={`/media/video/${selected.id}`} />
          ) : (
            <p id="watch-player-refused">
              This {selected.container.slice(1)} file will not open in a browser directly. The
              remux path that serves it as one ships with video.
            </p>
          )}
        </div>
      )}
    </section>
  );
}
