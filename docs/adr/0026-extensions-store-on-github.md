# 26. The extensions store reviews on GitHub, and the page only opens the catalogue

Date: 2026-10-08
Status: accepted, through the owner's request on 2026-10-08 that the review
home move to the public GitHub repository, and that the store page be how a
person opens the catalogue. It has no entry in the
[decision register](../decisions.md) yet; the owner adds one if he wants it
tracked there.

## Context

[Record 24](0024-official-extensions-repository.md) named a Forgejo repository
as the review home for extension records. The owner, on 2026-10-08, moved
that home to the public GitHub repository
`https://github.com/PremierStudio/gunmetal-extensions`. Record 22 still
forbids the client from fetching or running a plugin. SEC-EXT-018 still
forbids loading or executing plugin code. A pointer is not a downloader.

The product word is Extension. The door in the sidebar is Store. A plugin
would be code. An app would be a separate program. This is neither.

## Decisions

1. **The review home is**
   `https://github.com/PremierStudio/gunmetal-extensions`.
   `officialExtensionsRepository()` is that closed identity: forge
   `https://github.com`, owner `PremierStudio`, name `gunmetal-extensions`.
   This build still does not fetch or run it.
2. **A merged pull request there does not install itself.** The repository
   is where a record is reviewed. Gunmetal does not treat a merge as an
   install, and the store page has no action that fetches a package.
3. **The store page is how a person opens the catalogue.** `/store` shows
   the records already in this build. Opening a card opens the existing
   extension page on this server. It does not add a second detail page, and
   it does not download anything the record names.

## Consequences

The sidebar door is Store. The page says which records this server already
runs, and that a pull request does not install one. The Forgejo source of
truth for the player ([record 16](0016-forgejo-source-of-truth.md)) is
unchanged. Only the extensions review home moved. Creating that GitHub
repository and its access list is forge administration, not a player
feature.
