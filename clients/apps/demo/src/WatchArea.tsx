import { useCallback, useEffect, useState } from 'react';
import { loadSources, loadVideoDocument } from './video-api.ts';
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

/**
 * The Watch area: what the configured sources hold. Sources themselves are
 * added and rescanned in Settings → Libraries.
 */
export function WatchArea() {
  const [sources, setSources] = useState<SourcesDocument>({ sources: [] });
  const [document, setDocument] = useState<VideoDocument>({ kind: 'video', titles: [] });
  const [selected, setSelected] = useState<VideoTitle | undefined>(undefined);

  const reload = useCallback(async () => {
    const [readSources, readDocument] = await Promise.all([loadSources(), loadVideoDocument()]);
    setSources(readSources.ok ? readSources.value : { sources: [] });
    setDocument(readDocument.ok ? readDocument.value : { kind: 'video', titles: [] });
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  return (
    <section id="watch-area" aria-label="Watch">
      <header>
        <h1>Watch</h1>
        <p>Films and episodes, from the libraries in Settings.</p>
      </header>

      {document.titles.length === 0 ? (
        <p id="watch-grid-empty">
          {sources.sources.every((source) => source.kind === 'music')
            ? 'No movie or TV libraries yet. Add one in Settings → Libraries.'
            : 'No video files found in the configured libraries yet.'}
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
          <p>
            {metaOf(selected)} · {selected.container}
          </p>
          {browserPlayable(selected.container) ? (
            <video controls preload="metadata" src={`/media/video/${selected.id}`} />
          ) : (
            <p id="watch-player-refused">
              This {selected.container.slice(1)} file will not open in a browser directly. The remux
              path that serves it as one ships with video.
            </p>
          )}
        </div>
      )}
    </section>
  );
}
